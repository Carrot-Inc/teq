// Scala.js's execution contexts (`scala.scalajs.concurrent`): the queue runs each task as a
// microtask of the JS event loop, as the std's global one does, or after a timeout. What Scala.js's
// test bridge and a library's JavaScript code pass their futures.
package scala.scalajs.concurrent

import scala.concurrent.ExecutionContextExecutor

object QueueExecutionContext:
  def apply(): ExecutionContextExecutor = promises()

  def promises(): ExecutionContextExecutor = new ExecutionContextExecutor:
    def execute(runnable: Runnable): Unit =
      scala.concurrent.enqueueMicrotask: () =>
        try runnable.run()
        catch case t: Throwable => reportFailure(t)
    def reportFailure(cause: Throwable): Unit = cause.printStackTrace()

  def timeouts(): ExecutionContextExecutor = new ExecutionContextExecutor:
    def execute(runnable: Runnable): Unit =
      scala.scalajs.js.timers.setTimeout(0.0) {
        try runnable.run()
        catch case t: Throwable => reportFailure(t)
      }
    def reportFailure(cause: Throwable): Unit = cause.printStackTrace()

object JSExecutionContext:
  val queue: ExecutionContextExecutor = QueueExecutionContext()

  object Implicits:
    implicit val queue: ExecutionContextExecutor = JSExecutionContext.queue
