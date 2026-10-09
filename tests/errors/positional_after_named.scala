// A positional argument after a named one has to land on the parameter after the named one's;
// one that would skip a parameter is rejected, as in scalac.
def f(a: Int, b: Int = 2, c: Int = 3): String = s"$a $b $c"
final case class Item(title: String, value: Int, weight: Long = 1L)
@main def main(): Unit =
  println(f(b = 2, 1, a = 3))
  println(f(1, c = 3, 2))
  println(Item(value = 1, "t"))
  println(f(a = 1, 2, 3))
// expect: 6:11: error: positional after named argument
// expect: 7:11: error: positional after named argument
// expect: 8:11: error: positional after named argument
