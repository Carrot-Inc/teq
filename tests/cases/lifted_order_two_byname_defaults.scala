// Two local defs, each with a by-name default: each call's thunk calls its getter.
def mark(s: String): Int = { print(s); 1 }
@main def main(): Unit = {
  def p(a: => Int = mark("p"))(b: Int): Int = a + b
  def q(a: => Int = mark("q"))(b: Int): Int = a + b
  val k = () => 7
  println(p()(1) + q()(2) + k())
}
