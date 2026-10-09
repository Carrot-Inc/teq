// The second code pass's `float_decimal_rounding` program: the JDK's `FloatingDecimal` behind `new Float(String)`.
@main def run(): Unit =
  println(java.lang.Float.floatToIntBits(new java.lang.Float("1.0000000596046448").floatValue()))
