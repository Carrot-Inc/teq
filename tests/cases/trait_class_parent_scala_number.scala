//> using platform jvm
// split-watch: jvm
// A class whose first parent is `ScalaNumericConversions`, a library trait over the class `ScalaNumber`: its superclass
// is `ScalaNumber`, as scalac makes it (`Namer.ensureFirstIsClass`); on the JVM it was `Number`, the trait's class
// parent's own superclass, so the class was no `ScalaNumber`.
class Num(v: Int) extends scala.math.ScalaNumericConversions:
  def isWhole: Boolean = true
  def underlying: AnyRef = Integer.valueOf(v)
  def intValue: Int = v
  def longValue: Long = v.toLong
  def floatValue: Float = v.toFloat
  def doubleValue: Double = v.toDouble

@main def main(): Unit =
  val n = new Num(3)
  println(n.isInstanceOf[scala.math.ScalaNumber])
  println(n.toByte + " " + n.isValidInt + " " + n.toDouble)
