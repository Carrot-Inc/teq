// A zero `Int` remainder or floor modulus of a negative dividend is the Int zero, which widens
// to 0.0 and not -0.0, on JavaScript as on the JVM: the remainder by a constant, Scala's
// `Math.floorMod`, `scala.math.floorMod` and `java.lang.Math.floorMod`; a comparison of one
// tells no zero from another.
object K:
  final val k = -10

@main def run(): Unit =
  var x = -5
  var y = 5
  val r = x % 5
  println(1.0 / r.toDouble)
  println(1.0 / ((-5) % 5).toDouble)
  println(1.0 / (K.k % 5).toDouble)
  println(1.0 / (x % y).toDouble)
  var z = -10
  z %= 5
  println(1.0 / z.toDouble)
  println(1.0 / Math.floorMod(x, 5).toDouble)
  println(1.0 / scala.math.floorMod(x, 5).toDouble)
  println(1.0 / java.lang.Math.floorMod(x, 5).toDouble)
  println(1.0f / Math.floorMod(Int.MinValue, -1).toFloat)
  println(java.lang.Float.floatToIntBits(-Math.floorMod(Int.MinValue, -1).toFloat))
  println(java.lang.Float.floatToIntBits(-(x % 5).toFloat))
  println(1.0 / Math.floorMod(-5L, 5L).toDouble)
  println(1.0 / (x.toLong % 5L).toDouble)
  println(x % 3)
  println(x % 5 == 0 && x % 5 >= 0 && !(x % 5 < 0) && x % 2 != 0)
  println(Math.floorMod(x, 3))
  println(Math.floorMod(x, -3))
  println(scala.math.floorMod(-7L, 3L))
  println(try Math.floorMod(x, 0).toString catch { case e: ArithmeticException => "ArithmeticException" })
  println(try (x % (y - 5)).toString catch { case e: ArithmeticException => "ArithmeticException" })
