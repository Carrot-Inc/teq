// teq: --inline-substitution
// A case of an `inline match` specialised by what its type variables stand for before the calls
// in it resolve: `constValue[h]` of `case _: (h *: t)` is 1 for `1 *: EmptyTuple` and 2 for
// `2 *: EmptyTuple`, as scalac 3.8.4 gives them.
import scala.compiletime.{constValue, erasedValue}
inline def head[T <: Tuple]: Any = inline erasedValue[T] match
  case _: (h *: t) => constValue[h]
@main def run(): Unit =
  println(head[1 *: EmptyTuple])
  println(head[2 *: EmptyTuple])
