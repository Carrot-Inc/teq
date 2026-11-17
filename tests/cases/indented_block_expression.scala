// An indented block of one expression is that expression. A selection on the line that closes the block chooses
// among the overloads of the expression's method by the member selected, as a selection written on the
// expression's own line does: in a def's right side, in an operator's operand after a colon, in a case's body.
// A `yield` whose indented expression is the generator's own variable yields the collection itself, as the
// one-line form does; in braces the expression is a block's, and the collection is mapped. scalac prints the same
// lines.
trait T
trait U
object E { given T = new T {}; given U = new U {} }
import E.given
class C { def sel: String = "selected" }
object O:
  def f(using T): C = new C
  def f(using U): Int = 0

def rightSide: String =
  O.f
.sel

def operand: String = "X" `+`:
  O.f
.sel

def caseBody(x: Int): String = x match {
  case 1 =>
    O.f
  .sel
  case _ => "other"
}

@main def main(): Unit =
  println((rightSide, operand, caseBody(1), caseBody(2)))
  val xs = List(1, 2)
  val oneLine = for x <- xs yield x
  val indented = for x <- xs yield
    x
  val inBraces = for x <- xs yield { x }
  println((oneLine eq xs, indented eq xs, inBraces eq xs))
