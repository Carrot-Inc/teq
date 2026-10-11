//> using platform js
// java.util.Objects' equals and hashCode are Java's: a boxed number equals a box of its own class
// alone, compared by its bits, where Scala's `==` is cooperative. On JavaScript an Int and a Double
// are one number, as under Scala.js.
@main def run(): Unit =
  val i: Any = 1
  val l: Any = 1L
  val d: Any = 1.0
  println(java.util.Objects.equals(i, l))
  println(java.util.Objects.equals(l, 1L))
  println(java.util.Objects.equals(i, d))
  println(java.util.Objects.equals(0.0, -0.0))
  println(java.util.Objects.equals(Double.NaN, Double.NaN))
  println(java.util.Objects.equals(null, null))
  println(java.util.Objects.equals("a", null))
  println(i == l)
  println(java.util.Objects.hashCode(l))
  println(java.util.Objects.hashCode(d))
