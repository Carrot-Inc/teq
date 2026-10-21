// `adaptToSubType`'s constant conversion (`Constant.convertTo`) and `typedNumber`'s `Float` (the review of
// `cf6cad6c1`, item 5): the ascribed tree read as adapted, `(16777217L: Float).toInt` the constant 16777216; E167
// where the conversion loses digits. scalac 3.8.4 gives the error and the four warnings.
// expect: 9:19: error: type mismatch: found 16777216, required 16777217
// expect: 10:16: warning: Widening conversion from Int to Float loses precision.
// expect: 11:16: warning: Widening conversion from Long to Float loses precision.
// expect: 12:17: warning: Widening conversion from Long to Double loses precision.
// expect: 13:10: warning: Widening conversion from Long to Float loses precision.
val b: 16777217 = (16777217L: Float).toInt
val f: Float = 16777217
val h: Float = 16777217L
val d: Double = 9007199254740993L
val i = (16777217L: Float).toInt
