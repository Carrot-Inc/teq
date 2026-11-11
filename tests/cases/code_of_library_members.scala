//> using platform jvm
// std: scala-library
// `codeOf` of a library's parameterless members and of an extension's application on a literal, as dotty's printer
// shows the typer's tree: the member's reference retyped to its result (`adapt1`'s `ExprType` arm), no ascription of
// it, and the extension's application spanning its receiver and name. scalac prints the lines of the .expected file.
import scala.compiletime.codeOf
@main def run(): Unit =
  val s = "ab"
  println(codeOf(s.reverse))
  println(codeOf((s.reverse: String)))
  println(codeOf(s.reverse + "!"))
  println(codeOf(3.abs + 1))
  println(codeOf(s.charAt(1).isDigit))
