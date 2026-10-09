//> using platform js
// A negated plain inline call of a constant type among `Double`s harmonizes as its literal does,
// scalac's typer reading the constant type.
object M:
  inline def one: 1 = 1

def choose(b: Boolean) = if b then -M.one else 2.0

@main def main(): Unit =
  val x: Double = choose(true)
  println(x)
  val l: List[Double] = List(-M.one, 2.0)
  println(l)
