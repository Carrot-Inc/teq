// A case class with a private constructor has its mirror in its companion, which reaches the
// constructor, so a call site outside it gets the mirror; only a case object companion leaves
// the mirror to the call site (tests/errors/mirror_refusals).
import scala.deriving.Mirror
final case class Code private (sequential: Long, random: Int)
object Code:
  def of(s: Long, r: Int): Code = Code(s, r)
final case class Wrapper(value: Code)
inline def fields[T](using m: Mirror.ProductOf[T]): Int = scala.compiletime.constValue[Tuple.Size[m.MirroredElemTypes]]
object Main:
  def main(args: Array[String]): Unit =
    val m = summon[Mirror.ProductOf[Code]]
    println(m.fromProduct((1L, 2)))
    println(fields[Code] + fields[Wrapper])
    println(summon[Mirror.ProductOf[Wrapper]].fromProduct(Tuple1(Code.of(3, 4))))
