// jars: javafix
// targets: js interp jvm
// A Java final field with a `ConstantValue` is of its constant's type, as dotty's
// `ClassfileParser.parseAttributes` and `convertTo` read it: a class file's (javac's fixtures)
// and the JDK's the std stands in for (`Math.PI`, `Integer.MAX_VALUE`), so a read of one is the
// constant on every target, the Java class's body unneeded. A `float` constant is one too, and the lean
// std's `java.lang.Float` declares the JDK's finite ones as constants (`Float.MAX_VALUE`).
import fix.Generics

@main def main(): Unit =
  val limit: 42 = Generics.LIMIT
  val name: "generics" = Generics.NAME
  val big: 1099511627776L = Generics.BIG
  val pi: 3.141592653589793 = java.lang.Math.PI
  val e: 2.718281828459045 = Math.E
  val max: 2147483647 = Integer.MAX_VALUE
  val lmin: -9223372036854775808L = java.lang.Long.MIN_VALUE
  val top: '￿' = Character.MAX_VALUE
  inline val radix = Character.MAX_RADIX
  println(s"$limit $name $big $pi $e $max $lmin ${top.toInt} $radix")
  val fmax: 3.4028235e38f = Generics.FMAX
  val tenth: 0.1f = Generics.TENTH
  val jmax: 3.4028235e38f = java.lang.Float.MAX_VALUE
  val jmin: 1.4e-45f = java.lang.Float.MIN_VALUE
  val jnormal: 1.17549435e-38f = java.lang.Float.MIN_NORMAL
  val bits = List(fmax, tenth, jmax, jmin, jnormal).map(f => Integer.toHexString(java.lang.Float.floatToIntBits(f)))
  println(bits.mkString(" "))
