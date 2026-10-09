// teq: --max-inlines 80
//> using option -Xmax-inlines 80
// `--max-inlines` raises the inline nesting limit, as scalac's `-Xmax-inlines` does: forty
// nested expansions where the default stops at 32.
import scala.compiletime.*
object Main:
  inline def count[T <: Tuple]: Int = inline erasedValue[T] match
    case _: EmptyTuple => 0
    case _: (h *: t) => 1 + count[t]
  type T40 = (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)
  def main(args: Array[String]): Unit =
    println(count[T40])
