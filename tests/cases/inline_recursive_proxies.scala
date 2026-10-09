// A recursive inline method whose expansion nests another expansion of itself (through an
// inline given found by summonInline): the enclosing expansion keeps its own parameter
// bindings once the nested one is done, so `i + 1` names the enclosing `i`.
import scala.compiletime.*
import scala.deriving.Mirror
trait Show[A]:
  def show(a: A): String
object Show:
  given Show[Int] = (a: Int) => "int" + a
  given Show[String] = (a: String) => "str" + a
  inline given product[T <: Product](using m: Mirror.ProductOf[T]): Show[T] =
    val arr = new Array[Any](constValue[Tuple.Size[m.MirroredElemTypes]])
    fill[m.MirroredElemTypes](0, arr)
    (t: T) => t.productIterator.zip(arr.iterator).map((v, s) => s.asInstanceOf[Show[Any]].show(v)).mkString("(", ",", ")")
inline def fill[T <: Tuple](i: Int, arr: Array[Any]): Array[Any] = inline erasedValue[T] match
  case _: EmptyTuple => arr
  case _: (a *: b) =>
    arr(i) = summonInline[Show[a]]
    fill[b](i + 1, arr)
case class Pair(a: Int, b: String)
object Main:
  def main(args: Array[String]): Unit =
    val arr = new Array[Any](3)
    fill[(Int, Pair, String)](0, arr)
    println(arr(0).asInstanceOf[Show[Int]].show(1) + " " + arr(1).asInstanceOf[Show[Pair]].show(Pair(2, "x")) + " " + arr(2).asInstanceOf[Show[String]].show("s"))
