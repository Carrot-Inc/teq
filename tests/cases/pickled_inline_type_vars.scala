// An `inline match` whose cases bind type variables (`case _: (h *: t)`), a mirror's binder
// whose member type the body names (`p.MirroredElemTypes`) and a plain match's variable
// (`List[e]`), pickled with their `BIND`s, which scalac expands at each call. (A call of
// `arity` needs the mirror teq synthesizes at the call site, which the writer withholds.)
import scala.compiletime.erasedValue
import scala.deriving.Mirror

object Shapes:
  inline def count[T <: Tuple]: Int = inline erasedValue[T] match
    case _: EmptyTuple => 0
    case _: (h *: t) => 1 + count[t]
  inline def arity[A](using m: Mirror.Of[A]): Int = inline m match
    case s: Mirror.SumOf[A] => count[s.MirroredElemTypes]
    case p: Mirror.ProductOf[A] => -count[p.MirroredElemTypes]
  def plain(x: Any): Int = x match
    case l: List[e] => l.length
    case _ => 0

object TypeVars:
  def main(args: Array[String]): Unit =
    println(Shapes.count[(Int, String, Boolean)])
    println(Shapes.plain(List(1, 2)))
