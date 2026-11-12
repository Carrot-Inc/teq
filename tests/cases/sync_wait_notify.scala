//> using platform jvm
// wait, notify and notifyAll inside `synchronized`: a producer and a consumer over a one-slot buffer, a timed wait
// that returns with nothing notifying, and notifyAll waking every waiter; `wait` outside any block throws
// IllegalMonitorStateException, as the JVM's monitor rules have it. A watchdog ends the run at a deadline.
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

class Slot:
  var value: Option[Int] = None
  var released = false
  var woke = 0

object Main:
  def main(args: Array[String]): Unit =
    val watchdog = new Thread(() => { Thread.sleep(60000); println("deadline passed"); System.exit(2) })
    watchdog.setDaemon(true)
    watchdog.start()

    val slot = new Slot
    val items = 50
    val producer = Check.worker("producer") {
      for i <- 1 to items do
        slot.synchronized {
          while slot.value.nonEmpty do slot.wait()
          slot.value = Some(i)
          slot.notify()
        }
    }
    var sum = 0
    val consumer = Check.worker("consumer") {
      for _ <- 1 to items do
        slot.synchronized {
          while slot.value.isEmpty do slot.wait()
          sum += slot.value.get
          slot.value = None
          slot.notify()
        }
    }
    Check.join(producer)
    Check.join(consumer)
    Check.show(s"consumed $sum of ${items * (items + 1) / 2}", "consumed 1275 of 1275")

    val t0 = System.nanoTime
    slot.synchronized(slot.wait(50))
    Check.show("timed wait returned after its time: " + (System.nanoTime - t0 >= 40000000L), "timed wait returned after its time: true")

    val waiters = 4
    val waiting = new CountDownLatch(waiters)
    val sleepers = (1 to waiters).map(i => Check.worker(s"sleeper $i") {
      slot.synchronized {
        waiting.countDown()
        while !slot.released do slot.wait()
        slot.woke += 1
      }
    })
    Check.await(waiting, "every sleeper in its wait")
    slot.synchronized {
      slot.released = true
      slot.notifyAll()
    }
    sleepers.foreach(Check.join)
    Check.show(s"notifyAll woke ${slot.woke} of $waiters", "notifyAll woke 4 of 4")

    val outside =
      try
        slot.wait()
        "wait outside a block returned"
      catch case _: IllegalMonitorStateException => "wait outside a block: IllegalMonitorStateException"
    Check.show(outside, "wait outside a block: IllegalMonitorStateException")
    Check.done()
