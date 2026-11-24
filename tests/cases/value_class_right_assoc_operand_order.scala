// A right-associative extension a value class declares, called infix on its instance's import: the left operand is
// evaluated before the right one (scalac's `Typer.typedInfixOp` lifts it), though the method's class file takes the
// right one first. An impure left with an impure right, with a pure right, a pure left, two clauses and a using
// clause, and the call written out in its declared clause order.
class V(val n: Int) extends AnyVal {
  extension (x: Int) def +:(y: String): String = y + x + n
  extension (x: Int) def ++:(y: String)(z: Int): String = y + x + z + n
  extension (x: Int) def **:(y: String)(using k: Int): String = y + x + k + n
}

object Main {
  var log = List.empty[String]
  def num(k: Int): Int = { log = log :+ ("n" + k); k }
  def note(s: String): String = { log = log :+ s; s }
  def show(label: String, r: Any): Unit = { println(label + " " + r + " " + log.mkString(",")); log = Nil }
  given Int = 5

  def main(args: Array[String]): Unit = {
    val v = new V(7)
    import v.*
    show("both", num(1) +: note("a"))
    show("pure right", num(1) +: "a")
    show("pure left", 1 +: note("a"))
    show("two clauses", (num(1) ++: note("a"))(num(2)))
    show("using", num(1) **: note("a"))
    val any: Any = num(3) +: note("b")
    show("at Any", any)
    num(4) +: note("c")
    show("dropped", "")
    show("written out", v.+:(note("a"))(num(1)))
  }
}
