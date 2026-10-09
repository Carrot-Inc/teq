// expect: 22:11: error: None of the overloaded alternatives of method area in object Geometry with types
// expect:  (w: Double, h: Double): Double
// expect:  (side: Int): Int
// expect: match arguments (Boolean)
// expect: 23:11: error: Ambiguous overload. The overloaded alternatives of method mix in object Geometry with types
// expect:  (a: Long, b: Int): Int
// expect:  (a: Int, b: Long): Int
// expect: both match arguments (Int, Int)
// expect: 24:11: error: Ambiguous overload. The overloaded alternatives of method area in object Geometry with types
// expect: all match expected type Any
// expect: 25:11: error: None of the overloaded alternatives of method area in object Geometry with types
// expect: match arguments (Boolean, Boolean, Boolean)
// expect: 26:28: error: type mismatch: found String, required Double
// expect: 5 errors found
object Geometry:
  def area(side: Int): Int = side * side
  def area(side: String): Int = side.length
  def area(w: Double, h: Double): Double = w * h
  def mix(a: Int, b: Long): Int = 1
  def mix(a: Long, b: Int): Int = 2
@main def run(): Unit =
  println(Geometry.area(true))
  println(Geometry.mix(1, 2))
  println(Geometry.area)
  val r = Geometry.area(true, false, true)
  println(Geometry.area(1, "x"))
