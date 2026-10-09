// An overload set whose every alternative is incomplete is one incomplete method: its calls,
// with type arguments or arguments no alternative takes, have the error type silently.
object O:
  def f(x: Int) Int = x
  def f(x: String) String = x
  val r = f(1)
  val s = f[Int](1)
  def g[A](x: A) A = x
  def g[A](x: A, y: A) A = x
  val u = g(1, 2, 3)
  val v: String = g(1)
