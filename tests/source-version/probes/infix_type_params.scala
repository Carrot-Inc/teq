def f[Pair[A, B]](x: Int Pair String): Unit = ()
class K[Pair[A, B]] { def g(x: Int Pair String): Unit = () }
trait T {
  type M[A, B]
  infix type N[A, B]
  def h(x: Int M String): Unit
  def k(x: Int N String): Unit
}
infix type or[A, B] = Either[A, B]
type plain[A, B] = Either[A, B]
infix class Both[A, B]
class Neither[A, B]
object U {
  def a(x: Int or String): Unit = ()
  def b(x: Int plain String): Unit = ()
  def c(x: Int Both String): Unit = ()
  def d(x: Int Neither String): Unit = ()
}
trait More extends T {
  def n(x: Int N String): Unit
  def m(x: Int M String): Unit
}
