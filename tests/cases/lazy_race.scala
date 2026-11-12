//> using platform jvm
// A lazy val is computed once whatever the threads (dotty's LazyVals: a thread-safe member over its VarHandle, a
// local over its LazyRef holder's monitor): an object's member, a class's, a local and a top-level one, each read by
// eight threads released together while the initialiser waits until every thread has arrived at the read, so that
// the readers contend. Every reader gets the one value. A watchdog ends the run at a deadline.
import java.util.concurrent.{ConcurrentHashMap, ConcurrentLinkedQueue, CountDownLatch, TimeUnit}
import java.util.concurrent.atomic.AtomicInteger

// The run's acceptance: a worker's exception, a value other than the one expected, a wait or a state poll that passes
// its deadline, and a worker still running at the end each fail the run, which prints what failed and exits 1.
object Check:
  private val failures = new ConcurrentLinkedQueue[String]
  private val workers = new ConcurrentLinkedQueue[Thread]
  def fail(what: String): Unit = { failures.add(what); () }
  def that(ok: Boolean, what: String): Unit = if !ok then fail(what)
  /** Prints `line`, the run failing where it is not `expected`. */
  def show(line: String, expected: String): Unit =
    println(line)
    that(line == expected, s"expected `$expected`")
  /** A thread running `body`, joined by `done`; its exception fails the run. */
  def worker(name: String)(body: => Unit): Thread =
    val t = new Thread(() => try body catch case e: Throwable => fail(s"$name threw $e"))
    t.setName(name)
    workers.add(t)
    t.start()
    t
  def await(latch: CountDownLatch, what: String): Unit = that(latch.await(5, TimeUnit.SECONDS), s"deadline passed: $what")
  /** Waits until `reached` holds, the run failing where it does not within the deadline. */
  def poll(what: String)(reached: => Boolean): Unit =
    val end = System.nanoTime + 5000000000L
    while !reached && System.nanoTime < end do Thread.sleep(1)
    that(reached, s"not reached: $what")
  def join(t: Thread): Unit =
    t.join(10000)
    that(!t.isAlive, s"${t.getName} still running")
  /** Joins every worker, then exits 1 where anything failed. */
  def done(): Unit =
    val it = workers.iterator()
    while it.hasNext do join(it.next())
    if !failures.isEmpty then
      val f = failures.iterator()
      while f.hasNext do println("FAILED: " + f.next())
      System.exit(1)

final class Gate(val label: String, val threads: Int):
  val arrived = new AtomicInteger
  val runs = new AtomicInteger
  def enter(): Unit =
    runs.incrementAndGet()
    Check.poll(s"$label: every reader at the read")(arrived.get >= threads)

object Obj:
  val gate = new Gate("object member", 8)
  lazy val value: AnyRef = { gate.enter(); new Object }

class Cls:
  val gate = new Gate("class member", 8)
  lazy val value: AnyRef = { gate.enter(); new Object }

val topGate = new Gate("top-level", 8)
lazy val topValue: AnyRef = { topGate.enter(); new Object }

object Main:
  def race(gate: Gate, read: () => AnyRef): Unit =
    val start = new CountDownLatch(1)
    val seen = new ConcurrentHashMap[Int, AnyRef]
    val readers = (0 until gate.threads).map { i =>
      Check.worker(s"${gate.label} reader $i") {
        Check.await(start, "the start")
        gate.arrived.incrementAndGet()
        seen.put(i, read())
        ()
      }
    }
    start.countDown()
    readers.foreach(Check.join)
    val values = new java.util.IdentityHashMap[AnyRef, Unit]
    seen.values.toArray.foreach(v => values.put(v, ()))
    Check.show(s"${gate.label}: initialiser runs ${gate.runs.get}, readers ${seen.size}, distinct values ${values.size}", s"${gate.label}: initialiser runs 1, readers 8, distinct values 1")

  def main(args: Array[String]): Unit =
    val watchdog = new Thread(() => { Thread.sleep(60000); println("deadline passed"); System.exit(2) })
    watchdog.setDaemon(true)
    watchdog.start()
    race(Obj.gate, () => Obj.value)
    val c = new Cls
    race(c.gate, () => c.value)
    val localGate = new Gate("local", 8)
    lazy val local: AnyRef = { localGate.enter(); new Object }
    race(localGate, () => local)
    race(topGate, () => topValue)
    Check.done()
