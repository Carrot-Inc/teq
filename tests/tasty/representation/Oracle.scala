// The representation oracle's scalac (tests/tasty/representation/README.md): scalac 3.8.4 in one JVM, every job of
// the file given as the first argument, one per line, tab-separated:
//   version  <name>                                        the compiler's version
//   identity <name>                                        dotty/Identity.scala's lines
//   buckets  <name>                                        dotty/Buckets.scala's lines
//   api      <name>                                        dotty/Api.scala's lines: the quoted API's accessors
//   compile  <name> <class path> <out> <options> <file>... compiled to <out>, `FAIL` and the first line of each error
//   observe  <name> <class path> <out> <options> <file>... the same, then the text of every warning headed
//                                                          `OBSERVE` (observe.scala), in the source's order, without
//                                                          its heading, then under `== <name>-kinds` the kinds its
//                                                          arms reached
// Each job's output follows a line `== <name>`. Every line is printable ASCII (README.md, "Transport"): observe.scala's
// text already is, and a compiler message's line is written with every other character as `\uXXXX`.
import dotty.tools.dotc.Main
import dotty.tools.dotc.reporting.StoreReporter
import scala.io.Source

@main def oracle(jobs: String): Unit =
  for line <- Source.fromFile(jobs).getLines() if line.nonEmpty do
    val f = line.split('\t').toList
    println(s"== ${f(1)}")
    try f.head match
      case "version" => println(dotty.tools.dotc.config.Properties.versionNumberString)
      case "identity" => Identity.main(Array.empty)
      case "buckets" => dotty.tools.dotc.core.Buckets.main(Array.empty)
      case "api" => dotty.tools.dotc.core.Api.main(Array.empty)
      case kind => compile(f(1), f(2), f(3), f(4).split(' ').toList.filter(_.nonEmpty), f.drop(5), kind == "observe")
    catch case t: Throwable => println(ascii(s"FAIL threw $t"))
  sys.exit(0)

/** A line in printable ASCII, every other UTF-16 unit `\uXXXX`, the observer's text unchanged. */
def ascii(s: String): String =
  s.flatMap(c => if c >= ' ' && c < '\u007f' then c.toString else "\\u%04x".format(c.toInt))

def compile(name: String, cp: String, out: String, options: List[String], files: List[String], observe: Boolean): Unit =
  val reporter = new StoreReporter()
  java.nio.file.Files.createDirectories(java.nio.file.Paths.get(out))
  val classPath = if cp.isEmpty then out else cp
  Main.process((List("-usejavacp", "-color:never", "-classpath", classPath, "-d", out) ++ options ++ files).toArray, reporter)
  for d <- reporter.allErrors do println(ascii(s"FAIL ${d.message.split('\n').headOption.getOrElse("")}"))
  if observe then
    // In the source's order, a generated traversal's calls one by one (the reporter keeps its warnings newest first).
    val warnings = reporter.allWarnings.sortBy(d => if d.pos.exists then d.pos.start else -1)
    for d <- warnings if d.message.startsWith("OBSERVE\n") do d.message.split('\n').drop(1).map(ascii).foreach(println)
    // The kinds the observation's arms reached (observe.scala's `hit`), a section of their own: `<name>-kinds`.
    val kinds = reporter.allWarnings.filter(_.message.startsWith("OBSERVED-KINDS\n")).flatMap(_.message.split('\n').drop(1))
    println(s"== $name-kinds")
    kinds.distinct.sorted.map(ascii).foreach(println)
