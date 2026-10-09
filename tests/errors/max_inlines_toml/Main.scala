// expect: 11:13: error: Maximal number of successive inlines (36) exceeded,
// expect: 1 error found
// The `max-inlines` of the teq.toml sets the limit for the directory.
import scala.compiletime.*
object Main:
  inline def count[T <: Tuple]: Int = inline erasedValue[T] match
    case _: EmptyTuple => 0
    case _: (h *: t) => 1 + count[t]
  type T40 = (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)
  def main(args: Array[String]): Unit =
    println(count[T40])
