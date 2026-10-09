// A future's callback prepares its execution context when it is registered, as scala-library's
// `Transformation` does, and runs through the prepared one.
import scala.concurrent.*
import scala.concurrent.duration.*

object Main:
  def main(args: Array[String]): Unit =
    var prepared = 0
    val log = new StringBuilder
    val base = ExecutionContext.global
    val ec = new ExecutionContext:
      def execute(runnable: Runnable): Unit = throw new IllegalStateException("not prepared")
      def reportFailure(t: Throwable): Unit = ()
      override def prepare(): ExecutionContext =
        prepared += 1
        new ExecutionContext:
          def execute(runnable: Runnable): Unit =
            log.append("x")
            base.execute(runnable)
          def reportFailure(t: Throwable): Unit = ()
    val p = Promise[Int]()
    val mapped = p.future.map(_ + 1)(using ec).flatMap(v => Future.successful(v * 2))(using ec)
    p.success(1)
    val done = Promise[Unit]()
    mapped.onComplete { r =>
      println(s"$r prepared $prepared executed ${log.length}")
      done.success(())
    }(using base)
    try Await.ready(done.future, 10.seconds)
    catch case _: TimeoutException => ()
