import scala.quoted.*
class X(val v: Int)
class Z[A](val v: Int)
object Counter:
  private var n = 0
  transparent inline def next: Int = ${ step }
  def step(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
object X:
  transparent inline given x: X = ${ make }
  def make(using Quotes): Expr[X] =
    val v = Counter.step
    '{ new X($v) }
object Z:
  transparent inline given z[A]: Z[A] = ${ make[A] }
  def make[A: Type](using Quotes): Expr[Z[A]] =
    val v = Counter.step
    '{ new Z[A]($v) }
