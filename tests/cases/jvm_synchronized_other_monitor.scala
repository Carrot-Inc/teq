//> using platform jvm
// `synchronized` on another object's monitor, never on `this`: the block holds that object's monitor, `wait` and
// `notifyAll` on it are legal inside, and eight workers on one box lose no update.
import java.util.concurrent.{CountDownLatch, Executors}

class Box:
  var n = 0
  var ready = false

class Slot:
  @volatile var text = "not run"

// Every block here is on another object's monitor (the box's), never on `this`.
class Worker(box: Box):
  def bump(): Unit = box.synchronized {
    val seen = box.n
    Thread.onSpinWait()
    box.n = seen + 1
  }
  def held: String = box.synchronized { s"the box's ${Thread.holdsLock(box)}, its own ${Thread.holdsLock(this)}" }
  def await(): String =
    try box.synchronized { while !box.ready do box.wait(); "woken" }
    catch case e: IllegalMonitorStateException => "IllegalMonitorStateException"
  def signal(): String =
    try box.synchronized { box.ready = true; box.notifyAll(); "notified" }
    catch case e: IllegalMonitorStateException => "IllegalMonitorStateException"

@main def run(): Unit =
  val box = new Box
  println(s"inside a block on the box, a worker holds: ${new Worker(box).held}")
  val waited = new Slot
  val waiter = new Thread(new Runnable { def run(): Unit = waited.text = new Worker(box).await() })
  waiter.start()
  Thread.sleep(100)
  println(s"signal: ${new Worker(box).signal()}")
  waiter.join(5000)
  println(s"wait: ${waited.text}")
  val threads = 8
  val each = 20000
  val ex = Executors.newFixedThreadPool(threads)
  val start = new CountDownLatch(1)
  val done = new CountDownLatch(threads)
  for _ <- 1 to threads do
    ex.submit(new Runnable {
      def run(): Unit =
        val w = new Worker(box)
        try { start.await(); var i = 0; while i < each do { w.bump(); i += 1 } }
        finally done.countDown()
    })
  start.countDown(); done.await(); ex.shutdown()
  println(s"${box.n} of ${threads * each} increments kept by eight workers on one box's monitor")
