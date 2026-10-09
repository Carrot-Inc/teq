// A method of several parameters expected to be a function of one tuple is untupled, as a lambda
// is.

def key(a: String, i: Int): String = a + i
def three(a: Int, b: Int, c: Int): Int = a * 100 + b * 10 + c
final case class P(x: Int, y: String)
@main def main(): Unit =
  println(List(("a", 1), ("b", 2)).map(key))
  println(List((1, 2, 3), (4, 5, 6)).map(three))
  println(List((1, "x")).map(P.apply))
  val f: ((String, Int)) => String = key
  println(f(("c", 3)))
