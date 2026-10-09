import java.util.BitSet
import scala.quoted.*

// `java.util.BitSet` at compile time, as cats-parse builds its character classes for http4s'
// `uri"..."`: the std's body of the JDK's class runs in the interpreter on every target.
object Macros:
  def summary(bs: BitSet): String =
    s"$bs ${bs.cardinality()} ${bs.length()} ${bs.nextSetBit(3)} ${bs.nextClearBit(0)} ${bs.get(5)}"

  inline def bits(inline chars: String): String = ${ bitsImpl('chars) }
  def bitsImpl(chars: Expr[String])(using Quotes): Expr[String] =
    val s = chars.valueOrAbort
    val min = s.min.toInt
    val bs = new BitSet(s.max.toInt + 1 - min)
    s.foreach(c => bs.set(c.toInt - min))
    val range = new BitSet(8)
    range.flip(2, 6)
    range.clear(3)
    val other = new BitSet()
    other.set(4, 9)
    other.and(range)
    Expr(summary(bs) + " | " + summary(range) + " | " + summary(other))
