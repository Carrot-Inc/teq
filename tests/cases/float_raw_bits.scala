// `floatToRawIntBits` gives the bits a float holds, a NaN's sign and payload kept, where `floatToIntBits` gives
// Java's one NaN pattern. The NaNs here are quiet: a signalling one's bits are the processor's to keep or quiet.
object Main:
  def main(args: Array[String]): Unit =
    def raw(f: Float): Int = java.lang.Float.floatToRawIntBits(f)
    println(raw(1.0f))
    println(raw(-0.0f))
    println(raw(Float.NaN))
    println(raw(java.lang.Float.intBitsToFloat(0x7fc00001)))
    println(raw(java.lang.Float.intBitsToFloat(0xffc00000)))
    println(java.lang.Float.floatToIntBits(java.lang.Float.intBitsToFloat(0x7fc00001)))
    println(raw(Float.MinPositiveValue))
    println(raw(Float.PositiveInfinity))
    println(raw(-Float.MaxValue))
