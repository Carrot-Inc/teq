// A block's local class in a match type's scrutinee, the block's type a val's inferred type (the closure adversary of
// the pass on 861d8acf1, round 22, item 1's local-class shape): dotty's `TypeOps.avoid` (`AvoidMap`, an
// `ApproximatingTypeMap`) ranges `L` in the scrutinee, and `derivedMatchType`'s ranged-scrutinee arm (Types.scala
// 6859-6861) answers `range(bound.bounds.lo, bound.bounds.hi)`, the bound `String` exact, so `v: Inv[String]`:
// scalac 3.8.4 accepts and prints 1. teq keeps the local class in the block's type (`Inv[L match { case Int =>
// String }]`, as it keeps `Inv[L]` where scalac infers `Inv[? <: Object]`) and rejects: type mismatch: found
// Inv[L match { case Int => String }], required Inv[String]. With `L` in the case's pattern instead, scalac crashes
// (`MatchTypeCaseSpec.analyze`'s MatchError, repr_match_case_range_depfun_crash's).
class Inv[X](val x: Int)
@main def localClassMatchScrutineeAvoided(): Unit =
  val v = { class L; new Inv[L match { case Int => String }](1) }
  val w: Inv[String] = v
  println(w.x)
