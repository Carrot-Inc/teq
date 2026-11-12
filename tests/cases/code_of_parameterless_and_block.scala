// `codeOf` (and the inline val purity message) shows a tree as dotty's printer does: a parameterless method's
// reference without an argument list (dotty's `Select`), a block of expressions on lines of its own.
// scalac prints the lines of the .expected file.
import scala.compiletime.codeOf
object M:
  def one: 1 = 1
  def two(): Int = 2
  def three = 3
@main def run(): Unit =
  println(codeOf(M.one))
  println(codeOf(M.two()))
  println(codeOf(M.three + 1))
  println(codeOf({ println("hi"); 3 }))
