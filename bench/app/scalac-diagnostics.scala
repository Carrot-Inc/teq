//> using scala 3.8.4
//> using dep org.scala-lang::scala3-compiler:3.8.4

// scalac 3.8.4 with a reporter that writes each diagnostic it presents as one JSON line, for
// bench/app/diagnostics.py: `scala-cli run --server=false bench/app/scalac-diagnostics.scala --
// <out.jsonl> <scalac arguments>`. The console reporter's own output goes on as scalac prints it
// (its de-duplication of a position and its hiding of messages about erroneous types included,
// which the JSON lines follow), and the last line is `scalac: <errors> errors, <warnings>
// warnings`; exits 1 when there is an error. A diagnostic carries scalac's error code, its kind,
// its message, the point's 1-based line and column and the span's start and end (columns in
// UTF-16 units, as teq's answer counts them), the file as scalac was given it; one with no
// position has no file.

import dotty.tools.dotc.Driver
import dotty.tools.dotc.core.Contexts.Context
import dotty.tools.dotc.reporting.{ConsoleReporter, Diagnostic}

import java.io.{BufferedReader, InputStreamReader, PrintWriter}
import java.nio.charset.StandardCharsets.UTF_8
import java.nio.file.{Files, Paths}

object ScalacDiagnostics:
  private def quote(s: String): String =
    val b = new StringBuilder("\"")
    for c <- s do c match
      case '"' => b ++= "\\\""
      case '\\' => b ++= "\\\\"
      case '\n' => b ++= "\\n"
      case '\r' => b ++= "\\r"
      case '\t' => b ++= "\\t"
      case c if c < ' ' => b ++= f"\\u${c.toInt}%04x"
      case c => b += c
    b += '"'
    b.toString

  private def json(d: Diagnostic)(using Context): String =
    val severity = d.level match
      case 2 => "error"
      case 1 => "warning"
      case _ => "info"
    val number = d.msg.errorId.errorNumber
    val fields = Seq.newBuilder[(String, String)]
    fields += "severity" -> quote(severity)
    fields += "code" -> quote(if number >= 0 then f"E$number%03d" else "")
    fields += "id" -> quote(d.msg.errorId.toString)
    fields += "kind" -> quote(d.msg.kind.message)
    fields += "message" -> quote(d.msg.message)
    val pos = d.pos
    if pos.exists && pos.source.exists then
      fields += "file" -> quote(pos.source.file.path)
      fields += "line" -> (pos.line + 1).toString
      fields += "col" -> (pos.column + 1).toString
      fields += "startLine" -> (pos.startLine + 1).toString
      fields += "startCol" -> (pos.startColumn + 1).toString
      fields += "endLine" -> (pos.endLine + 1).toString
      fields += "endCol" -> (pos.endColumn + 1).toString
    fields.result().map((k, v) => s"${quote(k)}:$v").mkString("{", ",", "}")

  def main(args: Array[String]): Unit =
    if args.isEmpty then
      System.err.println("usage: scalac-diagnostics.scala <out.jsonl> <scalac arguments>")
      sys.exit(2)
    val lines = Files.newBufferedWriter(Paths.get(args(0)), UTF_8)
    val console = PrintWriter(System.out, true)
    val reporter = new ConsoleReporter(BufferedReader(InputStreamReader(System.in)), console, console):
      // The summary (`5 errors found`) is reported as a diagnostic without a position: no JSON line.
      private var summarizing = false
      override def printSummary()(using Context): Unit =
        summarizing = true
        try super.printSummary() finally summarizing = false
      override def doReport(dia: Diagnostic)(using Context): Unit =
        if !summarizing then
          lines.write(json(dia))
          lines.newLine()
        super.doReport(dia)
    new Driver().process(args.drop(1), reporter)
    lines.close()
    println(s"scalac: ${reporter.errorCount} errors, ${reporter.warningCount} warnings")
    sys.exit(if reporter.hasErrors then 1 else 0)
