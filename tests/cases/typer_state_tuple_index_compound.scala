// A plain inline call of a constant type as an operand of a tuple's index (`M.one + 0`, a negation,
// an ascribed operand) is its literal for the static index, as scalac's typer reads its constant
// type; one of a widened type (`M.wide`) is read at run time.
object M:
  inline def one: 1 = 1
  inline def minusOne: -1 = -1
  inline def wide: Int = 1

@main def run(): Unit =
  val a: String = (1, "two", true)(M.one + 0)
  val b: String = (1, "two", true)(-M.minusOne)
  val c: String = (1, "two", true)((M.one: 1) + 0)
  val d: String = (1, "two", true)(M.one)
  val e = (1, "two", true)(M.wide)
  println(a + b + c + d + e)
