// scala.concurrent's `Future` and `Promise` over the global execution context: a `Promise`
// completed and read before and after, `Await` of a completed future, and a chain of `map`,
// `flatMap`, `recover`, `recoverWith`, `sequence`, `traverse`, `zip`, `filter`, `failed`,
// `transform` and `andThen` printed from its completion callback. `main` waits for the chain on
// the JVM; on JavaScript nothing blocks, so `Await` of the incomplete future times out at once
// and the callbacks print once `main` returns.
import scala.concurrent.*
import scala.concurrent.duration.*
import scala.concurrent.ExecutionContext.Implicits.global
import scala.util.{Failure, Success, Try}

object Main:
  def main(args: Array[String]): Unit =
    val p = Promise[Int]()
    println(p.future.value)
    println(p.future)
    println(p.isCompleted)
    p.success(20)
    println(p.future.value)
    println(p.future)
    println(Try(p.success(1)).isFailure)
    println(p.trySuccess(2))
    println(Await.result(p.future, 1.second))
    println(Await.result(Future.successful("ready"), Duration.Inf))
    val done = Promise[Unit]()
    val log = new StringBuilder
    val chain =
      for
        a <- Future(6 * 7)
        b <- Future.successful(a + 1).map(_ * 2)
        c <- Future.failed[Int](new RuntimeException("boom")).recover { case e => e.getMessage.length }
        d <- Future.sequence(List(Future(1), Future(2), Future(3)))
        e <- Future.traverse(Vector(1, 2, 3))(x => Future(x * 10))
        z <- Future(1).zip(Future("b"))
        f <- Future.failed[Int](new IllegalStateException("x")).recoverWith { case _ => Future(5) }
        g <- Future(3).filter(_ > 5).failed.map(_.getMessage)
        h <- Future(4).transform(t => t.map(_ + 1))
        i <- p.future.flatMap(v => Future(v + 1))
        j <- Future(7).andThen { case Success(v) => log.append(s"andThen $v") }
        k <- Future[Int](throw new ArithmeticException("div")).failed.map(_.getClass.getSimpleName)
        l <- Future.fromTry(Try("t".toUpperCase))
      yield List(a, b, c, d, e, z, f, g, h, i, j, k, l)
    chain.onComplete {
      case Success(xs) =>
        xs.foreach(println)
        println(log)
        done.success(())
      case Failure(e) =>
        println("failed " + e)
        done.success(())
    }
    try Await.ready(done.future, 10.seconds)
    catch case _: TimeoutException => ()
