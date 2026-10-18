//> using platform jvm
// shutdownNow on a ScheduledThreadPoolExecutor returns the tasks not yet due and empties the queue, as JDK 24's
// ThreadPoolExecutor.drainQueue removes what the delay queue's drainTo leaves.
import java.util.concurrent.*
object Main:
  def main(args: Array[String]): Unit =
    val pool = new ScheduledThreadPoolExecutor(1)
    val f = pool.schedule(new Runnable { def run(): Unit = println("ran") }, 1L, TimeUnit.HOURS)
    val left = pool.shutdownNow()
    println("returned=" + left.size())
    println("queued=" + pool.getQueue().size())
    println("terminated=" + pool.awaitTermination(200L, TimeUnit.MILLISECONDS))
