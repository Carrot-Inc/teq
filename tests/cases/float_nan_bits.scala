// `floatToIntBits` and `doubleToLongBits` give every NaN Java's one pattern, whatever sign the
// engine left on it; Scala.js does the same.
object Main:
  def main(args: Array[String]): Unit =
    val nan = Float.NaN
    val dnan = Double.NaN
    println(java.lang.Float.floatToIntBits(-nan))
    println(java.lang.Float.floatToIntBits(nan))
    println(java.lang.Float.floatToIntBits(0.0f / 0.0f))
    println(java.lang.Float.floatToIntBits(-(0.0f / 0.0f)))
    println(java.lang.Float.floatToIntBits(-0.0f))
    println(java.lang.Float.floatToIntBits(1.0f))
    println(java.lang.Float.floatToIntBits(java.lang.Float.intBitsToFloat(0x7fc00001)))
    println(java.lang.Float.floatToIntBits(java.lang.Float.intBitsToFloat(0xffc00000)))
    println(java.lang.Float.intBitsToFloat(0x7fc00001).isNaN)
    println(java.lang.Double.doubleToLongBits(-dnan))
    println(java.lang.Double.doubleToLongBits(dnan))
    println(java.lang.Double.doubleToLongBits(0.0 / 0.0))
    println(java.lang.Double.doubleToLongBits(-(0.0 / 0.0)))
    println(java.lang.Double.doubleToLongBits(-0.0))
    println(java.lang.Double.doubleToLongBits(java.lang.Double.longBitsToDouble(0xfff8000000000000L)))
    println(java.lang.Double.doubleToLongBits(java.lang.Double.longBitsToDouble(0x7ff0000000000001L)))
    println(java.lang.Double.doubleToLongBits(Math.sqrt(-1.0)))
