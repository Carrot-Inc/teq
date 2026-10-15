// `NumericRange.count` tries the bounds and the step as `Int`s one after another, converting each
// only once the one before it round-trips, as scala-library's does: a custom `Integral`'s `toInt` is
// called on the start, which does not fit, and then on the count alone.
object ProbeIntegral extends Integral[Long]:
  override def zero: Long = 0L
  override def one: Long = 1L
  def plus(x: Long, y: Long): Long = x + y
  def minus(x: Long, y: Long): Long = x - y
  def times(x: Long, y: Long): Long = x * y
  def quot(x: Long, y: Long): Long = x / y
  def rem(x: Long, y: Long): Long = x % y
  override def negate(x: Long): Long = -x
  def fromInt(x: Int): Long = x.toLong
  def parseString(s: String): Option[Long] = Some(s.toLong)
  def toInt(x: Long): Int = { println("toInt " + x); x.toInt }
  def toLong(x: Long): Long = x
  def toFloat(x: Long): Float = x.toFloat
  def toDouble(x: Long): Double = x.toDouble
  def compare(x: Long, y: Long): Int = if x < y then -1 else if x > y then 1 else 0

@main def main(): Unit =
  println(List.range(2147483648L, 2147483650L, 1L)(using ProbeIntegral))
