//> using platform js
// Comparable.compareTo on the boxes of the primitives and on strings. Through a Comparable
// receiver Scala.js compares every number as a Double (-1 for two Shorts where the JDK
// subtracts); a statically typed box answers with its own class's compareTo.
final case class Version(n: Int) extends Ordered[Version]:
  def compare(that: Version): Int = n - that.n

@main def run(): Unit =
  val c: Comparable[Integer] = Integer.valueOf(3)
  println(c.compareTo(4))
  val s: Comparable[String] = "b"
  println(s.compareTo("a") + " " + s.compareTo("bcd"))
  val boxes: List[(Any, Any)] = List(
    (Integer.valueOf(3), Integer.valueOf(10)),
    (java.lang.Short.valueOf(3.toShort), java.lang.Short.valueOf(10.toShort)),
    (java.lang.Byte.valueOf(3.toByte), java.lang.Byte.valueOf(10.toByte)),
    (java.lang.Double.valueOf(0.0), java.lang.Double.valueOf(-0.0)),
    (java.lang.Double.valueOf(Double.NaN), java.lang.Double.valueOf(1.0)),
    (java.lang.Float.valueOf(1f), java.lang.Float.valueOf(2f)),
    (java.lang.Long.valueOf(5L), java.lang.Long.valueOf(7L)),
    (java.lang.Boolean.valueOf(true), java.lang.Boolean.valueOf(false)),
    (Character.valueOf('x'), Character.valueOf('a')),
    ("apple", "apricot"),
    (Version(2), Version(5)))
  println(boxes.map((a, b) => a.asInstanceOf[Comparable[Any]].compareTo(b)).mkString(" "))
  println(java.lang.Short.valueOf(3.toShort).compareTo(10.toShort) + " " + java.lang.Byte.valueOf(3.toByte).compareTo(10.toByte) + " " + Character.valueOf('x').compareTo('a'))
  println(java.lang.Float.valueOf(Float.NaN).compareTo(1f) + " " + java.lang.Float.valueOf(0f).compareTo(-0f))
  def largest[T <: Comparable[T]](xs: List[T]): T = xs.reduce((a, b) => if a.compareTo(b) >= 0 then a else b)
  println(largest(List(Integer.valueOf(3), Integer.valueOf(9), Integer.valueOf(4))) + " " + largest(List("x", "yz", "y")) + " " + largest(List(Version(1), Version(3))))
