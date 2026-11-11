package zma
import scala.deriving.Mirror
final case class C(x: Int)
object C { def noop = 1 }
final case class D(x: Int)
object Defs:
  inline def make[A](using m: Mirror.ProductOf[A]): A = m.fromProduct(Tuple1(3))
