// The compile-time evaluation of inline arguments never folds a value that JavaScript and the
// JVM would print differently: a string made from a Double or Float, a class name or a hash
// code stay computations. tests/fold.sh builds this program with and without the evaluation
// for both targets and compares what they print.
inline def twice(inline x: Int): Int = x + x
inline def show(inline s: String): String = "[" + s + "]"
inline def keep(inline d: Double): Double = d
inline def truth(inline b: Boolean): String = if b then "yes" else "no"

@main def run(): Unit =
  println(twice("abc".length + 1))
  println(show(List(1, 2).mkString(",")))
  println(show((1.0 / 3).toString))
  println(show(2.5.toString))
  println(show(List(1.5, 2.5).mkString(";")))
  println(show(s"${0.1 + 0.2}"))
  println(show("abc".hashCode.toString))
  println(keep(math.sqrt(2.0)))
  println(truth(math.sqrt(16.0) == 4.0))
  println(truth((1.0 / 3).toString.length > 3))
  println(show(1.5f.toString))
