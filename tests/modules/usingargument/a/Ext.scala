package uaa

// A using parameter passed as an ordinary argument (`mkString(sep)`): the product's body read back
// passes it to the clause the callee declares, an ordinary one, where a given argument alone once
// made a using application of it (`mkString()` applied to `sep`). Beside it, givens that do fill a
// using clause, written and inferred, of a String and of a Char.
extension (s: String)
  def f1(n: Int)(sep: String): String = List.fill(n)(s).mkString(sep)
  def f2(n: Int)(using sep: String): String = List.fill(n)(s).mkString(sep)
  def f3(n: Int)(using sep: String)(using c: Char): String = List.fill(n)(s).mkString(sep) + c
  def f4(n: Int)(using c: Char): String = List.fill(n)(s).mkString + c

def joined(xs: List[Int])(using sep: String): String = xs.mkString(sep)
def framed(xs: List[Int])(using sep: String): String = xs.mkString("[", sep, "]")
def relay(xs: List[Int])(using sep: String): String = joined(xs)(using sep)
def inferred(xs: List[Int])(using sep: String): String = joined(xs)

class Table(cells: Vector[String]):
  def row(using sep: String): String = cells.mkString(sep)
  def padded(width: Int)(using fill: Char): String = cells.map(_.padTo(width, fill)).mkString("|")
