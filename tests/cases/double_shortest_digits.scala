//> using platform js
// The digits of a Double and a Float as text where Java's rule and the plain shortest digits part:
// a small subnormal takes a second digit that is closer (4.9E-324, not 5.0E-324), and of two
// equally close shortest forms the even one is printed. Every position that renders a number:
// toString, Any, concatenation, s and raw interpolation, mkString, %s, the numeric conversions of
// format and the f interpolator. The JVM's text (scalac 3.8.4 on JDK 24, for the interpreter and
// the JVM) is in tests/jvm-expected; JavaScript prints the numbers as Scala.js does.
@main def main(): Unit =
  val ds = List(4.9E-324, 9.9E-324, 1.5E-323, 2.0E-323, 4.94E-322, 1.0E-320, 5.0E-310,
    2.225073858507201E-308, 2.2250738585072014E-308, 1.7976931348623157E308, Double.MinPositiveValue,
    2.0233473011568512E15, -9.041696664835262E14, 1.0977188341988792E15, 0.1 + 0.2, 1.0E23)
  for d <- ds do println(s"$d ${-d}")
  val fs = List(1.4E-45f, 2.8E-45f, 4.2E-45f, 7.0E-45f, 9.8E-45f, 1.0E-40f, 1.1754942E-38f,
    1.17549435E-38f, 3.4028235E38f, 2278536.2f, 3842041.2f, -1390857.2f, 0.1f)
  for f <- fs do println(s"$f ${-f}")
  val d = 4.9E-324
  val f = 1.4E-45f
  println(d)
  println(f)
  println(d.toString + " " + f.toString)
  println((d: Any))
  println((f: Any))
  println("d=" + d + " f=" + f)
  println(raw"$d $f")
  println(List(d, 1.0E-320).mkString(","))
  println(java.lang.Double.toString(d) + " " + java.lang.Float.toString(f))
  println(new StringBuilder().append(d).append(' ').append(f).toString)
  println("%s %s".format(d, f))
  println("%.2e".format(d))
  println("%e".format(d))
  println("%.3e".format(1.0E-320))
  println("%g".format(d))
  println("%.2e".format(2.2250738585072014E-308))
  println("%.1e".format(9.9E-324))
  println("%.3e".format(f))
  println(f"$d%.2e $d%s $f%.3e")
  println(java.lang.Math.nextUp(0.0) == d)
  println(java.lang.Math.nextDown(2.2250738585072014E-308))
