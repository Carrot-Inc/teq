// `ConstFold.ConstantTree` through a cast over an ascription (the review of `cf6cad6c1`, item 4): the cast is the
// outer tree, read by its type, never through to the ascription's constant inside it; a cast alone as before. scalac
// 3.8.4 gives these five errors (`@implicitNotFound`'s argument is no constant).
// expect: 10:15: error: type mismatch: found String, required "xy"
// expect: 11:12: error: type mismatch: found Int, required 2
// expect: 12:15: error: type mismatch: found Boolean, required true
// expect: 13:15: error: type mismatch: found String, required "xy"
// expect: 14:46: error: @implicitNotFound requires constant expressions as a parameter
def same[T](x: T): T = x
val s: "xy" = (same["x"]("x"): String).asInstanceOf[String] + "y"
val n: 2 = (1: Int).asInstanceOf[Int] + 1
val a: true = (null: String).asInstanceOf[String] == null
val c: "xy" = same["x"]("x").asInstanceOf[String] + "y"
type L = [@scala.annotation.implicitNotFound((same["x"]("x"): String).asInstanceOf[String] + "y") A] =>> Int
