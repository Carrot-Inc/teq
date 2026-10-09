// The second code pass's `float_strings` program: the JDK's `FloatingDecimal` behind `new Float(String)`.
@main def run(): Unit =
  for s <- List("-0.0", "0x1.8p1", "3.4028236e38", "1.4e-45", "NaN") do
    println(java.lang.Float.floatToIntBits(new java.lang.Float(s).floatValue()))
  try println(new java.lang.Float("bad")) catch case _: NumberFormatException => println("bad")
