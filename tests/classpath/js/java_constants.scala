// jars: javafix
// targets: js interp jvm
// A Java final field with a `ConstantValue` is of its constant's type, as dotty's
// `ClassfileParser.parseAttributes` and `convertTo` read it: a class file's (javac's fixtures)
// and the JDK's the std stands in for (`Math.PI`, `Integer.MAX_VALUE`), so a read of one is the
// constant on every target, the Java class's body unneeded.
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
