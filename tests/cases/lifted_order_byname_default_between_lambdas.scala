// A local def with a by-name default between two lambdas.
def mark(s: String): Int = { print(s); 1 }
@main def main(): Unit = {
  val f = () => mark("f")
  def loc(a: => Int = mark("D"))(b: Int): Int = a + b
  println(loc()(f()))
  val g = () => 2
  println(g())
}
