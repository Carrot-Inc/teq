// `adaptToSubType`'s `Constant.convertTo` against the expected type itself (the review of `4fd7f3293`, item 3): a union
// with a member other than `Null` is no class bound (`classBound`'s `stripNull` drops `Null` alone), so the constant is
// not converted, and with no view to it (`Byte`, `Short`, `Char`) the mismatch is E007, as scalac 3.8.4 gives it; a
// literal under such a union is an `Int` (`typedNumber`'s `isRef`), and `Float | String` takes `int2float`.
// expect: 10:24: error: type mismatch: found 65, required Byte | String
// expect: 11:26: error: type mismatch: found 65, required Short | Boolean
// expect: 12:22: error: type mismatch: found 65, required Char | Unit
// expect: 13:26: error: type mismatch: found 65, required Byte | String
def id[T](x: T): T = x
val b: Byte | String = id[65](65)
val s: Short | Boolean = id[65](65)
val c: Char | Unit = id[65](65)
val lit: Byte | String = 65
val f: Float | String = id[16777217](16777217)
