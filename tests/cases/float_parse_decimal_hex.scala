// `Float.parseFloat`, `new Float(String)` and `StringOps.toFloat` are the JDK's `FloatingDecimal.readJavaFormatString(s)
// .floatValue()`: a decimal rounded to the nearest float directly (never through the nearest double,
// which can sit on a float midpoint), the hexadecimal form with its round and sticky bits, signed
// zeros, infinities and NaN, subnormals, a trailing `f` or `d`, surrounding white space trimmed, and
// `NumberFormatException` for anything else.
object Main:
  def bits(s: String): String =
    try java.lang.Float.floatToIntBits(java.lang.Float.parseFloat(s)).toString
    catch case e: NumberFormatException => "NFE " + e.getMessage
  def main(args: Array[String]): Unit =
    println(java.lang.Float.floatToIntBits(new java.lang.Float("1.0000000596046448").floatValue()))
    println(new java.lang.Float("0x1.8p1").floatValue() == 3.0f)
    val inputs = List(
      "-0.0", "0x1.8p1", "3.4028236e38", "1.4e-45", "NaN", "-Infinity", "+Infinity", " 2.5f ", "7d",
      "1.0000000596046448", "1.00000005960464477539062500001", "0.1", "1e-46", "7e-46", "1.17549435E-38",
      "1.1754942E-38", "3.4028235e38", "3.40282356779733661637539395458142568448e38", "123456789",
      "1234567890123456789", "9.999999e9", "16777217", "16777219", "0.000001", "1e10", "1e11", "4.7e-38",
      "0x1p-149", "0x1p-150", "0x1.000001p0", "0x1.0000018p0", "0X.8P1", "0x1.P0", "-0x1.fffffep127",
      "0x1.ffffffp127", "0x1p128", "0x1p-99999999999", "0x0p0", "00012.5e+1", ".5", "5.", "1e+3",
      "", "bad", "1e", "1.2.3", "0x1.8", "--1", "1f2", "0x", "Infinit")
    for s <- inputs do println("[" + s + "] " + bits(s))
    try new java.lang.Float("bad") catch case e: NumberFormatException => println("bad " + e.getMessage)
    println(java.lang.Float.floatToIntBits("1.0000000596046448".toFloat) + " " + java.lang.Float.floatToIntBits("0x1.8p1".toFloat))
