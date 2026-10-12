// The equality of two numeric constants folds by value, as dotty's `ConstFold` compares them
// (`foldFloatOp`'s `floatValue ==`, `foldDoubleOp`'s): `0.0f == -0.0f` holds, no NaN equals
// itself, and numbers of two kinds compare in the wider one. A constant type's identity is still
// the value's bits.
object C:
  final val eq = 0.0f == -0.0f
  final val nan = (0.0f / 0.0f) == (0.0f / 0.0f)
  final val deq = 0.0 == -0.0
  final val dnan = (0.0 / 0.0) != (0.0 / 0.0)
  final val mixed = 1 == 1.0f
  final val long = 3L == 3.0
@main def run() =
  println(C.eq)
  println(C.nan)
  println(C.deq)
  println(C.dnan)
  println(C.mixed)
  println(C.long)
  val t: true = C.eq
  println(t)
