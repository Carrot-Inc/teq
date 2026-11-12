//> using platform jvm
// `synchronized` excludes (scalac's monitorenter/monitorexit, dotty's genSynchronized): threads incrementing plain vars
// under several locks and receivers at once lose no update, and no second entrant ever finds a block's `inside` flag
// set. Each read-modify-write yields inside the block now and then, so without the monitor updates are lost on every
// run. A watchdog ends the run at a deadline.
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

class Cell:
  var n = 0
  var inside = false

class Counter:
  val cell = new Cell
  def bump(violations: AtomicInteger, i: Int): Unit = this.synchronized(Main.step(cell, violations, i))

object Shared:
  val cell = new Cell
  def bump(violations: AtomicInteger, i: Int): Unit = synchronized(Main.step(cell, violations, i))

object Main:
  def step(c: Cell, violations: AtomicInteger, i: Int): Unit =
    if c.inside then violations.incrementAndGet()
    c.inside = true
    val v = c.n
    if i % 16 == 0 then Thread.`yield`()
    c.n = v + 1
    c.inside = false

  def main(args: Array[String]): Unit =
    val watchdog = new Thread(() => { Thread.sleep(60000); println("deadline passed"); System.exit(2) })
    watchdog.setDaemon(true)
    watchdog.start()
    val threads = 4
    val rounds = 4000
    val lock = new Object
    val locked = new Cell
    val counters = Array(new Counter, new Counter)
    val violations = new AtomicInteger
    val start = new CountDownLatch(1)
    val workers = (0 until threads).map { t =>
      Check.worker(s"worker $t") {
        Check.await(start, "the start")
        var i = 0
        while i < rounds do
          lock.synchronized(step(locked, violations, i))
          counters(t % 2).bump(violations, i)
          Shared.bump(violations, i)
          i += 1
      }
    }
    start.countDown()
    workers.foreach(Check.join)
    Check.show(s"lock ${locked.n} of ${threads * rounds}", "lock 16000 of 16000")
    Check.show(s"counters ${counters(0).cell.n} ${counters(1).cell.n} of ${threads * rounds / 2} each", "counters 8000 8000 of 8000 each")
    Check.show(s"object ${Shared.cell.n} of ${threads * rounds}", "object 16000 of 16000")
    Check.show(s"violations ${violations.get}", "violations 0")
    Check.done()
