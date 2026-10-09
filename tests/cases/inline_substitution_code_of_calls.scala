// `codeOf` shows an explicit `toString` as the call scalac's tree holds, written with or without
// its parentheses, where the conversion a concatenation renders an operand with is no call; an
// operand in parentheses where scalac's precedence asks for them (`"a" + (x + y)`, `s + x * y`).
// scalac prints the lines of the .expected file.
import scala.compiletime.codeOf
inline def code(inline x: String): String = codeOf(x)
def show(y: Int, d: Double): String =
  code(y.toString()) + " | " + code(y.toString) + " | " + code(d.toString) + " | " + code("a" + y)
def grouped(x: Int, y: Int, s: String): String =
  code("a" + (x + y)) + " | " + code("a" + x + y) + " | " + code(s + (x * y) + "b") + " | " + code((x + y).toString)
@main def run(): Unit =
  println(show(5, 2.5))
  println(grouped(1, 2, "s"))
