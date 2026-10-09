// jars: scala-library cats-kernel cats-core cats-effect-kernel-jvm cats-effect-std-jvm cats-effect-jvm cats-mtl-jvm
//> using dep org.typelevel::cats-effect:3.7.0
// cats-effect's `IOApp` object as the entry point: the trait's `private[this] var _runtime` and its
// `private lazy val queue` are the object's, `cats$effect$IOApp$$_runtime_$eq` storing the runtime
// (master threw `AbstractMethodError` in `IOApp.$init$`) and the queue's holder computing it once.
import cats.effect.*

object CApp extends IOApp:
  def run(args: List[String]): IO[ExitCode] =
    for
      _ <- IO.println("io app, " + args.size + " arguments")
      n <- IO(21).map(_ * 2)
      _ <- IO.println("result " + n)
    yield ExitCode.Success
