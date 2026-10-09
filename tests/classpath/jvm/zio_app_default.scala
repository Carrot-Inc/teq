// jars: scala-library zio-jvm zio-stacktracer-jvm zio-internal-macros-jvm izumi-reflect-jvm izumi-reflect-boopickle-jvm scala-collection-compat-jvm
//> using dep dev.zio::zio:2.1.26
// A `ZIOAppDefault` object as the entry point: `ZIOApp`'s and `ZIOAppDefault`'s vals are the
// object's fields, which their `$init$`s store in linearization order, and the `run` effect runs to
// completion, its arguments from `getArgs`, the exit code success.
import zio.*

object ZApp extends ZIOAppDefault:
  def run =
    for
      args <- getArgs
      _ <- Console.printLine("zio app, " + args.size + " arguments")
      n <- ZIO.succeed(21).map(_ * 2)
      _ <- Console.printLine("result " + n)
    yield ()
