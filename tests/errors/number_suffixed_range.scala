// expect: 9:9: error: number too large
// expect: 10:9: error: number too large
// expect: 11:9: error: number too small
// expect: 12:9: error: number too small
// expect: 13:11: error: number too large
// expect: 14:16: error: type mismatch: found Double, required Float
// A suffixed number out of its type's range is reported by the parser (dotc's `Parsers.literal`,
// `FromDigits`): infinite, or zero from nonzero digits; a minus before it is its sign.
val a = 1e40f
val b = 1e400d
val c = 1e-50f
val d = 1e-400d
val e = - 1e40f
val p: Float = -(1.5)
