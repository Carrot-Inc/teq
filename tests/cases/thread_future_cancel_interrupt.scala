//> using platform jvm
// A FutureTask cancelled with an interrupt while it runs: the runner keeps its interrupt status after
// run returns, as JDK 24's handlePossibleCancellationInterrupt leaves it.
import java.util.concurrent.{Callable, CountDownLatch, FutureTask}
object Main {
 def main(args: Array[String]): Unit = {
  val entered = new CountDownLatch(1)
  val task = new FutureTask[Unit](new Callable[Unit] {
   def call(): Unit = {
    entered.countDown()
    val end = System.nanoTime() + 3000000000L
    while (!Thread.currentThread().isInterrupted() && System.nanoTime() < end) Thread.`yield`()
   }
  })
  val worker = new Thread(() => { task.run(); println(Thread.currentThread().isInterrupted()) })
  worker.start()
  entered.await()
  task.cancel(true)
  worker.join()
 }
}
