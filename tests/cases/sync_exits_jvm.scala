//> using platform jvm
// split-watch: jvm
// Every way out of a `synchronized` block releases its monitor, exactly once (dotty's genSynchronized: monitorexit on
// the normal path, in the catch-any handler that rethrows, and in the cleanup an early return runs): a return from
// inside, from nested blocks, beside operands on the stack, through a `finally` inside and outside, an exception
// caught outside (after which another thread acquires the monitor at once), a non-local return from a lambda inside,
// and `boundary`/`break` across it through a `try` and through a local def.
import java.util.concurrent.{ConcurrentLinkedQueue, CountDownLatch, TimeUnit}
import scala.util.boundary, boundary.break

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

object Main:
  val lock = new Object
  val other = new Object

  def early(c: Boolean): Int = lock.synchronized { if c then return 5; 2 }
  def underOperands(c: Boolean): Int = 1 + lock.synchronized { if c then return 5; 2 }
  def nested(c: Boolean): Int = lock.synchronized { other.synchronized { if c then return 10; 20 } }
  def finallyInside(c: Boolean): Int = lock.synchronized { try { if c then return 1; 2 } finally () }
  def finallyOutside(c: Boolean): Int = try lock.synchronized { if c then return 3; 4 } finally ()
  def throwsInside(): Int = lock.synchronized(throw new RuntimeException("inside"))
  def nonLocal(xs: List[Int]): Int =
    xs.foreach(x => lock.synchronized { if x > 1 then return x })
    -1
  def breakThroughTry(): Int = boundary { try lock.synchronized { if lock ne null then break(3); 4 } finally () }
  def breakThroughDef(): Int = boundary {
    def f(): Int = lock.synchronized { if lock ne null then break(5); 6 }
    f()
  }

  def released(label: String, v: Any, expected: String): Unit =
    Check.show(s"$label $v held=${Thread.holdsLock(lock) || Thread.holdsLock(other)}", s"$label $expected held=false")

  def main(args: Array[String]): Unit =
    released("early", (early(true), early(false)), "(5,2)")
    released("operands", (underOperands(true), underOperands(false)), "(5,3)")
    released("nested", (nested(true), nested(false)), "(10,20)")
    released("finally inside", (finallyInside(true), finallyInside(false)), "(1,2)")
    released("finally outside", (finallyOutside(true), finallyOutside(false)), "(3,4)")
    released("exception", try throwsInside() catch case e: RuntimeException => e.getMessage, "inside")
    val acquired = new CountDownLatch(1)
    val t = Check.worker("acquirer")(lock.synchronized(acquired.countDown()))
    Check.show("another thread acquires at once: " + acquired.await(5, TimeUnit.SECONDS), "another thread acquires at once: true")
    Check.join(t)
    released("non-local return", nonLocal(List(1, 2, 3)), "2")
    released("break through try", breakThroughTry(), "3")
    released("break through def", breakThroughDef(), "5")
    Check.done()
