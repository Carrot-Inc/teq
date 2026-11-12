//> using platform jvm
// An exception thrown inside a `synchronized` block gives the monitor back (genSynchronized's handler: monitorexit, then
// the rethrow): the thread holds it inside and not after, another thread enters, and no update is lost where every
// hundredth call throws inside the block.
import java.util.concurrent.{CountDownLatch, Executors}
import java.util.concurrent.atomic.AtomicBoolean

object Vault:
  private val lock = new Object
  private var n = 0
  def heldNow: Boolean = Thread.holdsLock(lock)
  def fail(): Int = lock.synchronized {
    println(s"inside the block: holds the monitor: ${Thread.holdsLock(lock)}")
    throw new IllegalStateException("thrown inside")
  }
  // Every hundredth call throws after its increment; the monitor must be given back each time.
  def bump(i: Int): Unit = lock.synchronized {
    val seen = n
    Thread.onSpinWait()
    n = seen + 1
    if i % 100 == 0 then throw new IllegalStateException("thrown inside")
  }
  def value: Int = n
  def anotherThreadEnters(): Boolean =
    val got = new AtomicBoolean(false)
    val t = new Thread(new Runnable { def run(): Unit = lock.synchronized { got.set(true) } })
    t.start(); t.join(5000)
    got.get

@main def run(): Unit =
  try Vault.fail() catch case e: IllegalStateException => println(s"caught: ${e.getMessage}")
  println(s"after the throw: holds the monitor: ${Vault.heldNow}; another thread enters: ${Vault.anotherThreadEnters()}")
  val threads = 8
  val each = 20000
  val ex = Executors.newFixedThreadPool(threads)
  val start = new CountDownLatch(1)
  val done = new CountDownLatch(threads)
  for _ <- 1 to threads do
    ex.submit(new Runnable {
      def run(): Unit =
        try
          start.await()
          var i = 0
          while i < each do { try Vault.bump(i) catch { case _: IllegalStateException => () }; i += 1 }
        finally done.countDown()
    })
  start.countDown(); done.await(); ex.shutdown()
  println(s"${Vault.value} of ${threads * each} increments kept where every hundredth call throws inside the block")
