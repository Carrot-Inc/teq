// `S` of `scala.compiletime.ops.int` folds the successor of a natural number below `Int.MaxValue` alone
// (dotty's `TypeEval.tryCompiletimeConstantFold` through `natValue`): `S[-1]` and `S[Int.MinValue]` stay
// unreduced, and `compareS` relates them to no constant, so both values are refused as scalac refuses them.
// `S[Int.MaxValue]` overflows and is refused too; `S[1]` is 2.
import scala.compiletime.ops.int.S
val x: S[-1] = 0
val y: S[-2147483648] = -2147483647
val z: S[2147483647] = -2147483648
val ok: S[1] = 2
// expect: successor_natural.scala:6:16: error: type mismatch: found 0, required S[-1]
// expect: successor_natural.scala:7:25: error: type mismatch: found -2147483647, required S[-2147483648]
// expect: successor_natural.scala:8:24: error: type mismatch: found -2147483648, required S[2147483647]
// expect: 3 errors found
