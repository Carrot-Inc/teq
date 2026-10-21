// The constant's tag through `Constant.convertTo`, the adaptation, the ascription and `ConstantTree` (the review of
// `82464a1b7`, item 4): a constant narrowed to `Byte` or `Short` keeps its tag, which `foldUnop` folds no prefix operator
// of (`-`, `~`, `+`), so each is an `Int` against its literal type, E007 as scalac 3.8.4 gives it; `+` of a `Byte` or a
// `Char` is an `Int` (`def unary_+ : Int`); a binary operation, a conversion and a comparison of one fold (lines 22-24);
// a conversion's result (`65.toByte`, no fold of `foldedUnops`) is no constant, typed `Byte` or not (the last four).
// expect: 17:24: error: type mismatch: found Int, required -65
// expect: 18:20: error: type mismatch: found Int, required 65
// expect: 19:23: error: type mismatch: found Int, required -65
// expect: 20:22: error: type mismatch: found Int, required -66
// expect: 21:20: error: type mismatch: found Int, required 97
// expect: 25:22: error: type mismatch: found Int, required Byte
// expect: 26:23: error: type mismatch: found Int, required 65
// expect: 27:26: error: type mismatch: found Int, required 66
// expect: 28:30: error: type mismatch: found Int, required -65
// expect: 29:31: error: type mismatch: found Int, required 65
def id[T](x: T): T = x
val shortNegate: -65 = -(id[65](65): Short)
val bytePlus: 65 = +(65: Byte)
val shortMinus: -65 = -(65: Short)
val byteTilde: -66 = ~(65: Byte)
val charPlus: 97 = +'a'
val sum: 66 = (65: Byte) + 1
val widened: 65 = (id[65](65): Short).toInt
val compared: true = (65: Byte) == 65
val plusType: Byte = +(65: Byte)
val notConstant: 65 = 65.toByte.toInt
val notConstantSum: 66 = 65.toByte + 1
val notConstantNegate: -65 = -(65.toShort)
val notConstantAscribed: 65 = (65.toByte: Byte).toInt
