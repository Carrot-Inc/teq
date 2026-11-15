// The operand of an overloaded right-associative operator is typed ahead of the alternative's choice and bound
// where `typedInfixOp` lifts it: ahead of a receiver that is a `def` and of one that is a lazy value (master read
// the lazy value first).
class Over:
  def +:(x: Int): String = "int"
  def +:(x: String): String = "string"
lazy val lazyOver: Over = { print("r"); new Over }
def left: Int = { print("l"); 1 }
def word: String = { print("l"); "w" }
def over: Over = { print("r"); new Over }
@main def run(): Unit =
  println(left +: over)
  println(word +: over)
  println(left +: lazyOver)
