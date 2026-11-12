//> using platform jvm
// `this.synchronized` excludes (dotty's genSynchronized, monitorenter and monitorexit): eight threads incrementing a
// plain var inside the block lose no update and never meet inside it.
import java.util.concurrent.{CountDownLatch, Executors}

object Counter:
  private var n = 0
  private var inside = 0
  var overlaps = 0
  def bump(): Unit = this.synchronized {
    inside += 1
    if inside > 1 then overlaps += 1
    val seen = n
    Thread.onSpinWait()
    n = seen + 1
    inside -= 1
  }
  def value: Int = n

@main def run(): Unit =
  val threads = 8
  val each = 20000
  val ex = Executors.newFixedThreadPool(threads)
  val start = new CountDownLatch(1)
  val done = new CountDownLatch(threads)
  for _ <- 1 to threads do
    ex.submit(new Runnable { def run(): Unit = { start.await(); var i = 0; while i < each do { Counter.bump(); i += 1 }; done.countDown() } })
  start.countDown(); done.await(); ex.shutdown()
  println(s"${Counter.value} of ${threads * each} increments kept; two threads inside the block at once: ${if Counter.overlaps > 0 then "yes" else "no"}")
