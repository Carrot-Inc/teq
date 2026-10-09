//> using platform js
// Scala.js's execution contexts: the queue runs a task after the current one as a microtask,
// the timeouts' after the microtasks (the test bridge answers the adapter from the queue).
import scala.concurrent.Future
import scala.scalajs.concurrent.{JSExecutionContext, QueueExecutionContext}
import scala.scalajs.concurrent.JSExecutionContext.Implicits.queue

@main def run(): Unit =
  var order = List.empty[String]
  JSExecutionContext.queue.execute(() => order = "queue" :: order)
  QueueExecutionContext.timeouts().execute(() => println("timeout"))
  Future { println("queued") }
  println("sync " + order)
  Future.successful(1).map(_ + 1).foreach(v => println("mapped " + v))
