object Test {
  def f(x: Int): Int = x
  def f(x: String): String = x
  def g(h: Int => Int, x: Int): Int = h(x)
  def g(h: String => String, x: String): String = h(x)
  val result = g((f), 1)
}
// `(f)` is typed as `f`, with no expected type before the overload of g is chosen: scalac 3.8.4 reports the
// ambiguity (6:18, 0-based), where the bare `g(f, 1)` resolves.
// expect: 6:19: error: Ambiguous overload. The overloaded alternatives of method f in object Test with types
// expect: 1 error found
