//> using platform jvm
// split-watch: jvm
// A lazy val's state under Java serialisation (scala-library's LazyVals: Evaluating's and Waiting's writeReplace
// answer null): a computed value travels with its instance, while an instance serialised during its initialiser,
// alone or with a reader waiting, comes back uncomputed and computes the value again. Deadlines bound every wait.
import java.io.{ByteArrayInputStream, ByteArrayOutputStream, ObjectInputStream, ObjectOutputStream}
import java.util.concurrent.{ConcurrentLinkedQueue, CountDownLatch, TimeUnit}
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

object Phase:
  val calls = new AtomicInteger
  @volatile var entered = new CountDownLatch(0)
  @volatile var release = new CountDownLatch(0)
  def reset(): Unit =
    entered = new CountDownLatch(1)
    release = new CountDownLatch(1)

class Holder extends Serializable:
  lazy val value: String =
    val n = Phase.calls.incrementAndGet()
    Phase.entered.countDown()
    Check.await(Phase.release, s"the release of initialiser $n")
    s"value $n"

object Main:
  def copy[T](x: T): T =
    val bytes = new ByteArrayOutputStream
    val out = new ObjectOutputStream(bytes)
    out.writeObject(x)
    out.close()
    new ObjectInputStream(new ByteArrayInputStream(bytes.toByteArray)).readObject().asInstanceOf[T]

  def main(args: Array[String]): Unit =
    val computed = new Holder
    Check.show(s"computed: ${computed.value}, its copy: ${copy(computed).value}, calls=${Phase.calls.get}", "computed: value 1, its copy: value 1, calls=1")

    Phase.reset()
    val evaluating = new Holder
    val owner = Check.worker("evaluating owner") { evaluating.value; () }
    Check.await(Phase.entered, "evaluating: the initialiser entered")
    val duringInit = copy(evaluating)
    Phase.release.countDown()
    Check.join(owner)
    Check.show(s"serialised while evaluating: original ${evaluating.value}, copy ${duringInit.value}, calls=${Phase.calls.get}", "serialised while evaluating: original value 2, copy value 3, calls=3")

    Phase.reset()
    val waited = new Holder
    val first = Check.worker("waited owner") { waited.value; () }
    Check.await(Phase.entered, "waited: the initialiser entered")
    val second = Check.worker("waited reader") { waited.value; () }
    Check.poll("waited: the second reader waiting")(second.getState == Thread.State.WAITING)
    val duringWait = copy(waited)
    Phase.release.countDown()
    Check.join(first)
    Check.join(second)
    Check.show(s"serialised while a reader waits: original ${waited.value}, copy ${duringWait.value}, calls=${Phase.calls.get}", "serialised while a reader waits: original value 4, copy value 5, calls=5")
    Check.done()
