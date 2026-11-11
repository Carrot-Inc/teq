// `codeOf` of a call of an extension method the program defines, as dotty's printer shows the typer's tree: the
// method applied to its receiver (`Apply(Ident(twice), 3)`), the receiver an argument, not a path before the name.
import scala.compiletime.codeOf
object O:
  def p(x: Int): Int = x
extension (x: Int)
  def twice: Int = x * 2
  def plus(y: Int): Int = x + y
@main def run(): Unit =
  val m = 4
  println(codeOf(3.twice))
  println(codeOf(3.twice.twice))
  println(codeOf(m.twice))
  println(codeOf(O.p(3.twice)))
  println(codeOf(3.twice * 2))
  println(codeOf(3.plus(4)))
  println(codeOf(List(1).map(_.twice)))
