// An inline match on `erasedValue[E & Tuple]` with a tuple pattern (circe's derivation counts
// `et & Tuple`): the intersection matches a structural pattern one of its parts matches.
import scala.compiletime.*
inline def count[T <: Tuple]: Int = inline erasedValue[T] match
  case _: EmptyTuple => 0
  case _: (h *: ts) => 1 + count[ts]
inline def viaAnd[E]: Int = count[E & Tuple]
object Main:
  def main(args: Array[String]): Unit =
    println(count[(Int, String) & Tuple])
    println(viaAnd[(Int, String)])
