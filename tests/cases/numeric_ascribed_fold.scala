// `ConstantTree(Typed(expr, _))` reads the adapted tree (the review of `cf6cad6c1`, item 5): `(16777217L: Float)`'s
// operand is the constant converted by `adaptToSubType` (`Constant.convertTo`), 16777216f, so `.toInt` folds to
// 16777216; `(16777217: Float)` is `typedNumber`'s `Float`; both with scalac's E167 warning, which
// `tests/errors/lossy_constant_conversion.scala` holds and this program's build silences.
// teq: --wconf id=E167:s
val a: 16777216 = (16777217: Float).toInt
val b: 16777216 = (16777217L: Float).toInt
val c: 2L = (1: Long) + 1L

@main def run(): Unit =
  println(s"$a $b $c")
