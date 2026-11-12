//> using platform jvm
// A lazy val that reads itself while it is being computed. A local's initialiser runs under its holder's monitor,
// which the thread re-enters, so the read recomputes until the stack overflows (dotty's transformLocalDef). A
// member's read finds Evaluating, installs Waiting and awaits it on its own thread (dotty's mkThreadSafeDef): the
// reading thread signals that the initialiser was entered, then main sees it waiting on the latch within a deadline.
// That reader is the one thread the run does not join: it waits for ever, as a daemon, which the run asserts.
import java.util.concurrent.{ConcurrentLinkedQueue, CountDownLatch, TimeUnit}

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

class Recursive:
  val entered = new CountDownLatch(1)
  lazy val x: Int =
    entered.countDown()
    x + 1

object Main:
  def main(args: Array[String]): Unit =
    lazy val local: Int = local + 1
    Check.show(try local.toString catch case _: StackOverflowError => "local: stack overflow", "local: stack overflow")

    val r = new Recursive
    val reader = new Thread(() => try { r.x; () } catch case e: Throwable => Check.fail(s"the reader threw $e"))
    reader.setDaemon(true)
    reader.start()
    Check.show("initialiser entered: " + r.entered.await(5, TimeUnit.SECONDS), "initialiser entered: true")
    Check.poll("the reader waiting")(reader.getState == Thread.State.WAITING || !reader.isAlive)
    val frames = reader.getStackTrace.map(f => f.getClassName + "." + f.getMethodName)
    val onLatch = frames.exists(_.startsWith("java.util.concurrent.CountDownLatch.await"))
    val inLazyVals = frames.exists(_.startsWith("scala.runtime.LazyVals$Waiting")) || frames.exists(_.contains("x$lzyINIT"))
    Check.show(s"member: the reader waits on its own latch: ${reader.getState == Thread.State.WAITING && onLatch && inLazyVals}", "member: the reader waits on its own latch: true")
    Check.done()
