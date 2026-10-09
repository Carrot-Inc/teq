// jars: sttp-shared-core
//> using dep com.softwaremill.sttp.shared::core:1.5.2
// sttp's `FutureMonad` from its jar over the std's `scala.concurrent`: `blocking` reaches
// `scala.concurrent.package.blocking`, a package object's member selected through the package,
// `async` completes from a callback, `ensure` runs its finalizer, errors are handled; the
// results print from the completion callback, after which `main` stops waiting on the JVM.
import scala.concurrent.*
import scala.concurrent.duration.*
import scala.concurrent.ExecutionContext.Implicits.global
import scala.util.{Failure, Success}
import sttp.monad.{Canceler, FutureMonad}

object Main:
  def main(args: Array[String]): Unit =
    val m = new FutureMonad()
    val done = Promise[Unit]()
    var finalized = false
    val program =
      for
        a <- m.blocking(20 + 1)
        b <- m.map(m.unit(a))(_ * 2)
        c <- m.async[Int] { cb =>
          cb(Right(b + 1))
          Canceler(() => ())
        }
        d <- m.handleError(m.error[Int](new RuntimeException("boom"))) { case e => m.unit(e.getMessage.length) }
        e <- m.ensure(m.eval(c + d), m.eval { finalized = true })
      yield List(a, b, c, d, e)
    program.onComplete { r =>
      r match
        case Success(xs) => println(xs.mkString(" "))
        case Failure(t) => println("failed " + t)
      println(s"finalized $finalized")
      done.success(())
    }
    try Await.ready(done.future, 10.seconds)
    catch case _: TimeoutException => ()
