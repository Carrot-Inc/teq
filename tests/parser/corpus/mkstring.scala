// mkString without arguments is an overload in Scala. It must not turn into a function value
// now that methods named without arguments are eta-expanded.
import scala.collection.mutable.ArrayBuffer

extension (n: Int)
  def plus(m: Int): Int = n + m

@main def run(): Unit =
  val xs = List(1, 2, 3)
  println(xs.mkString)
  println(xs.mkString(","))
  println(xs.mkString("[", ",", "]"))
  val s = xs.mkString
  println(s.length)
  println(Vector("a", "b").mkString)
  println(Array(1, 2).mkString)
  println(Map(1 -> "a", 2 -> "b").mkString)
  println(Map(1 -> "a", 2 -> "b").mkString(", "))
  println(Set(7).mkString)
  println(ArrayBuffer(1, 2).mkString)
  println("x" + xs.map(_ * 2).mkString + "y")
  println(s"${xs.mkString}!")
  // an extension that needs arguments still becomes a function value
  val p = 1.plus
  println(p(2))
