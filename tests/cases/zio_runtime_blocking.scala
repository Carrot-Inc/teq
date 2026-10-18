//> using dep dev.zio::zio:2.1.26
//> using platform jvm
// jars: zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
// split-watch: jvm
// a blocking effect's failure as the effect's error; several blocking effects in parallel.
import zio.*

object Main extends ZIOAppDefault:
  def run =
    for
      name <- ZIO.attemptBlocking(Thread.currentThread().getName)
      _ <- Console.printLine("blocking thread: " + name.startsWith("zio-default-blocking-"))
      e <- ZIO.attemptBlocking(throw new IllegalStateException("blocked")).either
      _ <- Console.printLine(e.left.map(_.getMessage).toString)
      sum <- ZIO.foreachPar(1 to 4)(i => ZIO.attemptBlocking { Thread.sleep(50); i }).map(_.sum)
      _ <- Console.printLine("sum " + sum)
    yield ()
