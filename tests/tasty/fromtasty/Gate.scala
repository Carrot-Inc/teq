// scalac 3.8.4 reading teq's TASTy: every job of the file given
// as the first argument, one per line, tab-separated, run in this one JVM:
//   read  <name>  <class path>  <.tasty>...           -from-tasty, the outline admitted, every
//                                                      right-hand side forced (CompilationUnit's
//                                                      forceTrees) and the tree checker run
//                                                      after readTasty, which stops the run
//   readjs <name> <class path>  <.tasty>...           `read` as scalac compiling for Scala.js
//                                                      (`-scalajs`) reads, js.| a union type
//   regen <name>  <class path>  <out dir>  <.tasty>... the same compiled to class files in
//                                                      <out dir>, the checker after every phase
// Prints `ok <name>` or `FAIL <name>` with the first lines of what scalac reported or threw; a
// job without a result after two minutes fails as a loop in the reader and the next one starts.
import dotty.tools.dotc.Main
import dotty.tools.dotc.reporting.StoreReporter
import scala.io.Source

@main def gate(jobs: String): Unit =
  for line <- Source.fromFile(jobs).getLines() if line.nonEmpty do
    val result = new java.util.concurrent.atomic.AtomicReference[Option[String]](Some("no result in 120 s: the reader loops"))
    val worker = new Thread(() => result.set(try run(line) catch case t: Throwable => Some(s"the job's line: $t")))
    worker.setDaemon(true)
    worker.start()
    worker.join(120000)
    val name = line.split('\t')(1)
    result.get match
      case None => println(s"ok $name")
      case Some(what) => println(s"FAIL $name\n$what")
  sys.exit(0)

def run(line: String): Option[String] =
  val f = line.split('\t').toList
  val (kind, cp, rest) = (f(0), f(2), f.drop(3))
  val common = List("-from-tasty", "-Xallow-outline-from-tasty", "-Ycheck:all", "-usejavacp", "-classpath", cp, "-color:never")
  val args = kind match
    case "read" => common ++ List("-Ystop-after:readTasty") ++ rest
    case "readjs" => common ++ List("-scalajs", "-Ystop-after:readTasty") ++ rest
    case "regen" => common ++ List("-d", rest.head) ++ rest.tail
  val reporter = new StoreReporter()
  val outcome =
    try
      Main.process(args.toArray, reporter)
      val errors = reporter.allErrors
      if errors.isEmpty then None
      else Some(errors.take(5).map(d => s"${d.pos.source}:${d.pos.line + 1}: ${d.message.linesIterator.take(3).mkString(" / ")}").mkString("\n"))
    catch case t: Throwable =>
      val trace = t.getStackTrace.take(4).map(e => s"    at $e").mkString("\n")
      Some(s"${t.getClass.getName}: ${String.valueOf(t.getMessage).linesIterator.take(3).mkString(" / ")}\n$trace")
  outcome
