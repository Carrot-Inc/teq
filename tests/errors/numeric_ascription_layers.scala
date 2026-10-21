// `ConstantTree` over an adapted ascription (the review of `9e9b35b3d`, item 4): `((1f: Float): Double)`'s inner tree is
// the `Float` ascription widened to `Double`, a conversion read by its type, no constant; the negative answer kept. scalac
// 3.8.4 gives the three E007; `(1f: Float).toDouble.toInt`, a constant, is accepted.
// expect: 8:16: error: type mismatch: found Int, required 1
// expect: 9:23: error: type mismatch: found Int, required 16777216
// expect: 11:19: error: type mismatch: found Int, required 1
// absent: numeric_ascription_layers.scala:10:
val small: 1 = ((1f: Float): Double).toInt
val large: 16777216 = ((16777217L: Float): Double).toInt
val control: 1 = (1f: Float).toDouble.toInt
val intLayer: 1 = ((1: Int): Double).toInt
