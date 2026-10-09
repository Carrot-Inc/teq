// The extensions of an instance tried in turn over one argument cache: a literal is typed
// against each candidate's own formal, never cached as the first candidate typed it (a `Double`
// literal there, which the second's `Int` formal does not take). scalac resolves the overload
// on the arguments typed alone and prints `int:1`; a cached `1.0` was "found Double, required
// Int".
trait Ops:
  extension [T](x: Int) def combine(y: Double, z: Boolean): String = "double:" + y.toInt
  extension [T](x: Int) def combine(y: Int, z: Int): String = "int:" + y
  extension [T](x: Int) def wide(y: Long, z: Boolean): String = "long:" + y
  extension [T](x: Int) def wide(y: Int, z: String): String = "int:" + y + z
  extension [T](x: Int) def pair(y: (Double, Double), z: Boolean): String = "doubles:" + y
  extension [T](x: Int) def pair(y: (Int, Int), z: Int): String = "ints:" + y
object O extends Ops

@main def run(): Unit =
  println(O.combine[Unit](1)(1, 1))
  println(O.combine[Unit](1)((1: Int), 1))
  println(O.combine[Unit](1)(-1, 2))
  println(O.combine[Unit](1)((3), 2))
  println(O.combine[Unit](1)(2, true))
  println(O.wide[Unit](1)(5, "!"))
  println(O.wide[Unit](1)(5, false))
  println(O.pair[Unit](1)((1, 2), 3))
