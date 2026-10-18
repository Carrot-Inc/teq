//> using platform jvm
// A single-thread scheduled executor: a periodic task ticks three times and is cancelled, a delayed
// value is computed after its delay, a delayed task cancelled before it runs answers its `get` with
// `CancellationException`; after `shutdown` and `awaitTermination` nothing ticks.
import java.util.concurrent.{CancellationException, CountDownLatch, Executors, TimeUnit}
import java.util.concurrent.atomic.AtomicInteger

object Main:
  def main(args: Array[String]): Unit =
    val ex = Executors.newSingleThreadScheduledExecutor()
    val ticks = new AtomicInteger
    val three = new CountDownLatch(3)
    val f = ex.scheduleAtFixedRate(() => { ticks.incrementAndGet(); three.countDown() }, 0, 10, TimeUnit.MILLISECONDS)
    three.await()
    println(f.cancel(false))
    println(s"${f.isCancelled} ${f.isDone}")
    val later = ex.schedule[Int](() => 42, 20, TimeUnit.MILLISECONDS)
    println(later.get)
    val never = ex.schedule[String](() => "ran", 10, TimeUnit.SECONDS)
    println(never.cancel(false))
    try never.get
    catch case e: CancellationException => println("get: CancellationException")
    ex.shutdown()
    println(ex.awaitTermination(1, TimeUnit.SECONDS))
    println(s"${ex.isShutdown} ${ex.isTerminated}")
    val at = ticks.get
    Thread.sleep(30)
    println(ticks.get == at)
