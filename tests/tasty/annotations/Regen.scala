// The annotations' check (check.sh): each job of the file given, `warn <name> <class path> <out
// dir> <.tasty>...`, compiled to class files from TASTy alone under -deprecation; prints each
// warning as `<line>: <message>`, or `FAIL` with what scalac reported or threw.
import dotty.tools.dotc.Main
import dotty.tools.dotc.reporting.StoreReporter
import scala.io.Source

@main def regen(jobs: String): Unit =
  for line <- Source.fromFile(jobs).getLines() if line.nonEmpty do
    val f = line.split('\t').toList
    val (cp, out, tastys) = (f(2), f(3), f.drop(4))
    val reporter = new StoreReporter()
    try
      Main.process((List("-from-tasty", "-Xallow-outline-from-tasty", "-deprecation", "-usejavacp", "-color:never", "-classpath", cp, "-d", out) ++ tastys).toArray, reporter)
      for d <- reporter.allErrors do println(s"FAIL ${d.message.linesIterator.next()}")
      for d <- reporter.allWarnings.sortBy(_.pos.line) do println(s"${d.pos.line + 1}: ${d.message.linesIterator.next()}")
    catch case t: Throwable => println(s"FAIL $t")
