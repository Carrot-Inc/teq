// An inline method's proxy for a `Mirror.ProductOf[T]` parameter: the mirror object it stands for
// is read through the refinement of the parameter's type, so `mc.fromProduct` is a `T`, also
// inside a lambda and an `inline match`.
import scala.deriving.Mirror

object Mapper:
  inline def direct[Out <: Product](using mc: Mirror.ProductOf[Out]): Out = mc.fromProduct(Tuple1(1))
  inline def lambda[In, Out <: Product](using mc: Mirror.ProductOf[Out]): In => Out = t => mc.fromProduct(Tuple1(t))
  inline def matched[In, Out <: Product](using mc: Mirror.ProductOf[Out]): In => Out =
    t =>
      inline scala.compiletime.erasedValue[In] match
        case _: Tuple => mc.fromProduct(t.asInstanceOf[Tuple])
        case _        => mc.fromProduct(Tuple1(t))
  inline def matchedNoLambda[In, Out <: Product](t: In)(using mc: Mirror.ProductOf[Out]): Out =
    inline scala.compiletime.erasedValue[In] match
      case _: Tuple => mc.fromProduct(t.asInstanceOf[Tuple])
      case _        => mc.fromProduct(Tuple1(t))

case class One(id: Int)
case class Pair(id: Int, name: String)

object Main:
  def main(args: Array[String]): Unit =
    println(Mapper.direct[One])
    println(Mapper.lambda[Int, One](2))
    println(Mapper.matched[(Int, String), Pair]((3, "c")))
    println(Mapper.matched[Int, One](4))
    println(Mapper.matchedNoLambda[(Int, String), Pair]((5, "e")))
