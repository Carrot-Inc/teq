//> using platform jvm
// A scheduled pool's delayed-task policy disabled after shutdown cancels the task it no longer keeps and lets the
// pool terminate, as JDK 24's setter runs onShutdown, with its termination check.
import java.util.concurrent.*
object Main:
  def main(args: Array[String]): Unit =
    val pool = new ScheduledThreadPoolExecutor(1)
    val future = pool.schedule(new Runnable { def run(): Unit = () }, 1, TimeUnit.HOURS)
    pool.shutdown()
    pool.setExecuteExistingDelayedTasksAfterShutdownPolicy(false)
    println("cancelled=" + future.isCancelled())
    println("terminated=" + pool.awaitTermination(100, TimeUnit.MILLISECONDS))
    pool.shutdownNow()
