// expect: float_decimal_literals.scala:13:16: error: number too large
// expect: float_decimal_literals.scala:14:16: error: number too small
// expect: float_decimal_literals.scala:15:16: error: number too large
// expect: float_decimal_literals.scala:16:9: error: number too large
// expect: float_decimal_literals.scala:17:17: error: number too small
// expect: float_decimal_literals.scala:18:16: error: type mismatch: found Double, required Float
// expect: float_decimal_literals.scala:20:16: error: type mismatch: found Double, required Float
// expect: float_decimal_literals.scala:21:27: error: type mismatch: found Double, required Float
// expect: float_decimal_literals.scala:23:11: error: type mismatch: found Double, required Float
// expect: float_decimal_literals.scala:24:
// An unsuffixed decimal out of the expected type's range, and the `Double`s no `Float` takes; a
// tuple's element through a branch as well (scalac reports the literal, teq the `if`: the line alone).
val a: Float = 1e40
val b: Float = 1e-50
val c: Float = -3.5e38
val e = 1e400
val g: Double = 1e-400
val h: Float = 1.5d
final val d = 1.5
val i: Float = d
val t: (Float, Double) = (1.1, 1.1)
def p(q: (Float, Int)): Float = q._1
val r = p(1.5, 2)
val tb: (Float, Int) = (if true then 1.1 else 2.2, 2)
