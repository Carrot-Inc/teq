// lila's routers, as its sbt build has liplay's routes compiler write them, run by teq as the
// overlay's TeqCommand (overlay/LilaTeq.scala) under `teq interp` with the compiler's jars on the
// class path: play.routes.compiler's own `RoutesCompiler.compile` over each routes file, the
// injected generator, the forward or the reverse routers, from the build's root so that the files'
// `@SOURCE` lines name them as sbt's do (`conf/routes`).
//
// teq interp bench/lila/routes.scala --classpath <routes compiler jars> -- <forward> <reverse> <routes files...> <output>
import java.io.File
import play.routes.compiler.{InjectedRoutesGenerator, RoutesCompiler}
import play.routes.compiler.RoutesCompiler.RoutesCompilerTask

@main def routes(args: String*): Unit =
  if args.length < 3 then
    System.err.println("usage: routes.scala <forward> <reverse> <routes files...> <output>")
    sys.exit(2)
  val forward = args(0).toBoolean
  val reverse = args(1).toBoolean
  val output = new File(args.last)
  for path <- args.slice(2, args.length - 1) do
    val task = RoutesCompilerTask(new File(path), Nil, forward, reverse, false)
    RoutesCompiler.compile(task, InjectedRoutesGenerator, output) match
      case Right(_) => ()
      case Left(errors) =>
        errors.foreach(e => System.err.println(s"${e.source}:${e.line.getOrElse(0)}: ${e.message}"))
        sys.exit(1)
