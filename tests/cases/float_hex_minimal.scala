// The second code pass's `float_hex_minimal` program: the JDK's `FloatingDecimal` behind `new Float(String)`.
@main def run(): Unit =
  try println(new java.lang.Float("0x1.8p1").floatValue() == 3.0f)
  catch case _: NumberFormatException => println("bad")
