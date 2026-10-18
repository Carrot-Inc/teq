//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// An `Unsafe.unsafe` run on `Runtime.default`: a fiber forked to sleep 200 ms through the clock's
// scheduler and joined by its parent, which `unsafe.run` waits for on the main thread; then two
// fibers interleaving by their sleeps.
import zio.*

object Main:
  def main(args: Array[String]): Unit =
    Unsafe.unsafe { implicit u =>
      println(Runtime.default.unsafe.run(ZIO.sleep(200.millis).fork.flatMap(_.join)))
      Runtime.default.unsafe.run(
        for
          f <- (ZIO.sleep(100.millis) *> Console.printLine("child")).fork
          _ <- Console.printLine("parent")
          _ <- f.join
        yield ()
      ).getOrThrowFiberFailure()
    }
