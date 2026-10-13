// scala-library's numeric companions declare their bounds as constants (`final val MaxValue = 0x7fffffff`,
// `final val MaxValue = java.lang.Float.MAX_VALUE`), so a value of the constant's singleton type takes one,
// as it takes the JDK's (`Integer.MAX_VALUE`); the lean std declares them so.
object Limits:
  final val limit = Int.MaxValue

@main def main(): Unit =
  val imax: 2147483647 = Int.MaxValue
  val imin: -2147483648 = Int.MinValue
  val lmax: 9223372036854775807L = Long.MaxValue
  val lmin: -9223372036854775808L = Long.MinValue
  val cmax: '￿' = Char.MaxValue
  val cmin: '\u0000' = Char.MinValue
  val fmax: 3.4028235e38f = Float.MaxValue
  val fmin: -3.4028235e38f = Float.MinValue
  val ftiny: 1.4e-45f = Float.MinPositiveValue
  val dmax: 1.7976931348623157e308 = Double.MaxValue
  val dmin: -1.7976931348623157e308 = Double.MinValue
  val dtiny: 4.9e-324 = Double.MinPositiveValue
  println(s"$imax $imin $lmax $lmin ${cmax.toInt} ${cmin.toInt}")
  println(List(fmax, fmin, ftiny).map(f => Integer.toHexString(java.lang.Float.floatToIntBits(f))))
  println(List(dmax, dmin, dtiny).map(d => java.lang.Long.toHexString(java.lang.Double.doubleToLongBits(d))))
  val again: 2147483647 = Limits.limit
  var widened = Int.MaxValue
  widened -= 1
  println(s"$again $widened")
