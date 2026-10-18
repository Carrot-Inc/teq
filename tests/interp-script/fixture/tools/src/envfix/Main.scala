package envfix

import cats.syntax.all.*
import com.monovore.decline.{Command, Opts}
import java.io.FileInputStream
import java.net.{InetAddress, InetSocketAddress, ServerSocket, Socket}
import zio.*
import zio.interop.catz.*
import zio.json.*

// A command-line tool of the shape of a development environment's script: a home directory with a JSON
// registry, a child process whose log is read back through fs2, a port opened and polled, fibers raced.
enum Cmd:
  case Init(home: String)
  case Add(home: String, name: String, port: Int)
  case ListAll(home: String)
  case Log(home: String)
  case Probe
  case Race
  case Teardown(home: String)

object Cli:
  private val home = Opts.argument[String]("home")

  val command: Command[Cmd] = Command("envfix", "a fixture of a ZIO command-line tool"):
    Opts.subcommand("init", "create the home directory and an empty registry")(home.map(Cmd.Init.apply)) <+>
      Opts.subcommand("add", "add an entry to the registry")((home, Opts.argument[String]("name"), Opts.argument[Int]("port")).mapN(Cmd.Add.apply)) <+>
      Opts.subcommand("list", "list the registry's entries")(home.map(Cmd.ListAll.apply)) <+>
      Opts.subcommand("log", "run a child writing a log, then read the log back")(home.map(Cmd.Log.apply)) <+>
      Opts.subcommand("probe", "open a port and poll it until it answers")(Opts.unit.as(Cmd.Probe)) <+>
      Opts.subcommand("race", "race sleeping fibers")(Opts.unit.as(Cmd.Race)) <+>
      Opts.subcommand("teardown", "remove the home directory")(home.map(Cmd.Teardown.apply))

final case class Entry(name: String, port: Int) derives JsonCodec

object Registry:
  def path(home: os.Path): os.Path = home / "registry.json"

  def read(home: os.Path): Task[List[Entry]] =
    ZIO.attemptBlocking(os.read(path(home))).flatMap(text => ZIO.fromEither(text.fromJson[List[Entry]]).mapError(e => new RuntimeException("corrupt registry: " + e)))

  // Written beside and moved into place, as a registry other runs read is.
  def write(home: os.Path, entries: List[Entry]): Task[Unit] = ZIO.attemptBlocking {
    val next = home / "registry.json.next"
    os.write.over(next, entries.toJsonPretty)
    os.move(next, path(home), replaceExisting = true, atomicMove = true)
  }

object Main extends ZIOAppDefault:
  private def at(home: String): os.Path = os.Path(home, os.pwd)

  private def execute(cmd: Cmd): Task[Unit] = cmd match
    case Cmd.Init(home) =>
      for
        _ <- ZIO.attemptBlocking(os.makeDir.all(at(home) / "envs"))
        _ <- Registry.write(at(home), Nil)
        names <- ZIO.attemptBlocking(os.list(at(home)).map(_.last).sorted)
        _ <- Console.printLine("initialised with " + names.mkString(", "))
      yield ()
    case Cmd.Add(home, name, port) =>
      for
        entries <- Registry.read(at(home))
        _ <- Registry.write(at(home), entries :+ Entry(name, port))
        _ <- Console.printLine(s"added $name on $port (${entries.size + 1} entries)")
      yield ()
    case Cmd.ListAll(home) =>
      Registry.read(at(home)).flatMap {
        case Nil => Console.printLine("no entries")
        case entries => ZIO.foreachDiscard(entries)(e => Console.printLine(f"${e.name}%-8s ${e.port}%5d"))
      }
    case Cmd.Log(home) =>
      val log = at(home) / "child.log"
      for
        status <- ZIO.attemptBlocking {
          val child = new ProcessBuilder("sh", "-c", "echo first; echo second; echo third")
            .redirectOutput(log.toIO)
            .redirectInput(ProcessBuilder.Redirect.from(new java.io.File("/dev/null")))
            .start()
          child.waitFor()
        }
        lines <- fs2.io.readInputStream[Task](ZIO.attemptBlocking(new FileInputStream(log.toIO)), 16)
          .through(fs2.text.utf8.decode)
          .through(fs2.text.lines)
          .filter(_.nonEmpty)
          .compile
          .toList
        _ <- Console.printLine(s"child exited $status, ${lines.size} lines, the last: ${lines.last}")
      yield ()
    case Cmd.Probe =>
      ZIO.scoped {
        for
          server <- ZIO.acquireRelease(ZIO.attemptBlocking(new ServerSocket(0, 1, InetAddress.getByName("127.0.0.1"))))(s => ZIO.succeed(s.close()))
          port = server.getLocalPort
          _ <- ZIO.attemptBlocking {
                 val socket = new Socket()
                 socket.connect(new InetSocketAddress("127.0.0.1", port), 500)
                 socket.close()
               }.retry(Schedule.spaced(20.millis) && Schedule.recurs(20))
          _ <- Console.printLine("the port answered")
          taken <- ZIO.attemptBlocking(new ServerSocket(port, 1, InetAddress.getByName("127.0.0.1")).close()).isFailure
          _ <- Console.printLine("taken while open: " + taken)
        yield port
      }.flatMap { port =>
        ZIO.attemptBlocking(new ServerSocket(port, 1, InetAddress.getByName("127.0.0.1")).close()).isSuccess.flatMap(free => Console.printLine("free once closed: " + free))
      }
    case Cmd.Race =>
      for
        winner <- ZIO.sleep(30.millis).as("fast").race(ZIO.sleep(2.seconds).as("slow"))
        _ <- Console.printLine("winner: " + winner)
        all <- ZIO.foreachPar(List(3, 1, 2))(n => ZIO.sleep((n * 20).millis).as(n))
        _ <- Console.printLine("in order: " + all.mkString(","))
        late <- ZIO.sleep(2.seconds).as("late").timeout(40.millis)
        _ <- Console.printLine("timed out: " + late.isEmpty)
        fiber <- (ZIO.sleep(20.millis) *> ZIO.succeed(42)).fork
        n <- fiber.join
        _ <- Console.printLine("joined " + n)
      yield ()
    case Cmd.Teardown(home) =>
      ZIO.attemptBlocking(os.remove.all(at(home))) *> Console.printLine("removed")

  def run: URIO[ZIOAppArgs, Unit] =
    for
      args <- getArgs
      code <- Cli.command.parse(args) match
        case Right(cmd) => execute(cmd).foldZIO(e => Console.printLineError("envfix: " + e.getMessage).orDie.as(ExitCode.failure), _ => ZIO.succeed(ExitCode.success))
        case Left(help) if help.errors.isEmpty => Console.printLine(help.toString).orDie.as(ExitCode.success)
        case Left(help) => Console.printLineError(help.toString).orDie.as(ExitCode(1))
      _ <- exit(code)
    yield ()
