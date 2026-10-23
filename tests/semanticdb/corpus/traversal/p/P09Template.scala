package p09
class Ann(x: Int) extends scala.annotation.StaticAnnotation
trait T[A] { def t: A }
abstract class Base(val k: Int)
@Ann(1) class C[X](@Ann(2) val a: Int, private[p09] var b: X, c: String)(using d: Int) extends Base(a), T[X]:
  self: T[X] =>
  def t: X = b
  private[this] val e = 1
  protected[p09] def f = c
  def this(x: Int) =
    this(x, null.asInstanceOf[X], "")(using 1)
  @Ann(3) def g = this.a + e
  class Inner
  end Inner
end C
class K[T: Ordering](t: T)
