// scala-library's `Numeric` is an `Ordering`: a numeric type class is an ordering, its `equiv` the
// ordering's, which a `NumericRange` asks for its emptiness and an override answers (izumi's
// `LTTOrdering`, whose `equiv` an export keeps once anything asks an ordering's).
import scala.collection.immutable.NumericRange

object Loud extends Integral[Long]:
  override def zero: Long = 0L
  override def one: Long = 1L
  def plus(x: Long, y: Long): Long = x + y
  def minus(x: Long, y: Long): Long = x - y
  def times(x: Long, y: Long): Long = x * y
  def quot(x: Long, y: Long): Long = x / y
  def rem(x: Long, y: Long): Long = x % y
  override def negate(x: Long): Long = -x
  def fromInt(x: Int): Long = x.toLong
  def parseString(s: String): Option[Long] = s.toLongOption
  def toInt(x: Long): Int = x.toInt
  def toLong(x: Long): Long = x
  def toFloat(x: Long): Float = x.toFloat
  def toDouble(x: Long): Double = x.toDouble
  def compare(x: Long, y: Long): Int = java.lang.Long.compare(x, y)
  override def equiv(x: Long, y: Long): Boolean =
    println(s"equiv $x $y")
    x == y

@main def main(): Unit =
  println(NumericRange(3L, 3L, 1L)(using Loud).isEmpty)
  val ord: Ordering[Long] = Loud
  println(ord.equiv(2L, 2L))
  println(List(3L, 1L, 2L).sorted(using Loud))
