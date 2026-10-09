// jars: zio zio-stacktracer zio-internal-macros izumi-reflect izumi-reflect-boopickle scala-collection-compat macrotask-executor scala-java-time
//> using dep dev.zio::zio:2.1.26
// zio 2.1.26's runtime across asynchronous boundaries, as a `ZIOAppDefault` so that both platforms
// wait for it: `fork`/`join`, `ZIO.fromFuture`, `ZIO.async` completed from a timer fiber, a
// `Queue` between two fibers, `sleep`, and a `Schedule` with a delay (`spaced` and `recurs`).
package zioasync

import zio.*
import scala.concurrent.Future

object Main extends ZIOAppDefault:
  def fromCallback(n: Int): UIO[Int] =
    ZIO.async[Any, Nothing, Int] { cb =>
      Unsafe.unsafe { implicit u =>
        Runtime.default.unsafe.fork(ZIO.sleep(5.millis) *> ZIO.succeed(cb(ZIO.succeed(n * 10))))
      }
      ()
    }

  def run =
    for
      f1 <- ZIO.succeed(21).delay(10.millis).fork
      f2 <- ZIO.succeed(2).fork
      a <- f1.join
      b <- f2.join
      _ <- Console.printLine(s"joined ${a * b}")
      fut <- ZIO.fromFuture(implicit ec => Future(6 * 7))
      _ <- Console.printLine(s"future $fut")
      cb <- fromCallback(4)
      _ <- Console.printLine(s"async $cb")
      q <- Queue.bounded[String](4)
      producer <- ZIO.foreachDiscard(List("x", "y", "z"))(s => q.offer(s) *> ZIO.sleep(1.milli)).fork
      got <- ZIO.foreach(1 to 3)(_ => q.take)
      _ <- producer.join
      _ <- Console.printLine(s"queue ${got.mkString(",")}")
      ref <- Ref.make(0)
      n <- ref.updateAndGet(_ + 1).repeat(Schedule.spaced(2.millis) && Schedule.recurs(3))
      total <- ref.get
      _ <- Console.printLine(s"schedule $n $total")
      slept <- ZIO.sleep(20.millis).as("slept").timeout(1.second)
      _ <- Console.printLine(s"timeout $slept")
      r <- ZIO.fail("once").retry(Schedule.recurs(2)).either
      _ <- Console.printLine(s"retry $r")
    yield ()
