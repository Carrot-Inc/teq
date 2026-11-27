// A local def with a default inside another local def, called with its default there: the getter is asked while the
// outer def's body is lowered, and its place, the inner def's, precedes the outer def's.
@main def main(): Unit = {
  def outer(x: Int): Int = { def inner(a: Int = 1)(b: Int) = a + b; inner()(x) }
  val k = () => 4
  println(outer(2) + k())
}
