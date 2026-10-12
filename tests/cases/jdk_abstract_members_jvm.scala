//> using platform jvm
// split-watch: jvm
// The JDK's abstract `Reader` and `Number` members implemented without `override`, as on scalac (the lean std's
// stand-ins of these two declare them otherwise).
class Empty extends java.io.Reader:
  def read(cbuf: Array[Char], off: Int, len: Int): Int = -1
  def close(): Unit = ()
class Seven extends java.lang.Number:
  def intValue(): Int = 7
  def longValue(): Long = 7L
  def floatValue(): Float = 7f
  def doubleValue(): Double = 7.0
@main def run(): Unit =
  println(new Empty().read())
  val n = new Seven
  println(n.intValue() + n.longValue() + n.byteValue())
