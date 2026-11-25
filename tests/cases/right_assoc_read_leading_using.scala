// An infix right-associative extension with a leading using clause: the left operand is bound before the using
// argument is searched and before the right operand runs.
def n(): Int = { println("left"); 1 }
def r(): Int = { println("right"); 2 }
@main def run(): Unit =
  given u: String = { println("using"); "u" }
  extension (using x: String)(l: Int)
    def *:(r: Int): Int = { println("body"); l + r }
  println(n() *: r())
