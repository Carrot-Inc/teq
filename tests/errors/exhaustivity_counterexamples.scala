// teq: --wconf any:error
// The uncovered cases an exhaustivity warning shows, and their order, are dotty's (`checkExhaustivity`,
// Space.scala 933–952): the cases subtracted last first (an `Or` folded by prepending), a type taken as its
// cases wherever something is subtracted from it, what is left split lazily in its order, deduplicated, and
// the first six shown; a match whose every case is guarded leaves its scrutinee whole. A part two ways reach is
// listed twice, as `flatten` lists it (`unreachable`'s `(C, B, B)`), and the warning stands at the scrutinee, so
// that an unreachable case's warning does not hide it. Each message of exhaustivity_counterexamples.check is
// scalac 3.8.4's.
sealed trait T0
case object A0 extends T0
case object B0 extends T0
sealed trait T1
case object A1 extends T1
case object B1 extends T1
sealed trait T2
case object A2 extends T2
case object B2 extends T2
sealed trait T3
case object A3 extends T3
case object B3 extends T3
sealed trait T4
case object A4 extends T4
case object B4 extends T4
sealed trait T5
case object A5 extends T5
case object B5 extends T5
sealed trait T6
case object A6 extends T6
case object B6 extends T6
sealed trait T7
case object A7 extends T7
case object B7 extends T7
sealed trait T8
case object A8 extends T8
case object B8 extends T8
sealed trait T9
case object A9 extends T9
case object B9 extends T9
sealed trait T10
case object A10 extends T10
case object B10 extends T10
sealed trait T11
case object A11 extends T11
case object B11 extends T11
sealed trait T12
case object A12 extends T12
case object B12 extends T12
sealed trait T13
case object A13 extends T13
case object B13 extends T13
sealed trait T14
case object A14 extends T14
case object B14 extends T14
sealed trait T15
case object A15 extends T15
case object B15 extends T15
sealed trait T16
case object A16 extends T16
case object B16 extends T16
sealed trait T17
case object A17 extends T17
case object B17 extends T17
sealed trait T18
case object A18 extends T18
case object B18 extends T18
sealed trait T19
case object A19 extends T19
case object B19 extends T19
case class R20(x0: T0, x1: T1, x2: T2, x3: T3, x4: T4, x5: T5, x6: T6, x7: T7, x8: T8, x9: T9, x10: T10, x11: T11, x12: T12, x13: T13, x14: T14, x15: T15, x16: T16, x17: T17, x18: T18, x19: T19)
def wide(r: R20): Int = r match
  case R20(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, A15, A16, A17, A18, A19) => 0
  case R20(B0, B1, B2, B3, B4, B5, B6, B7, B8, B9, B10, B11, B12, B13, B14, B15, B16, B17, B18, B19) => 1
  case R20(B0, A1, B2, A3, B4, A5, B6, A7, B8, A9, B10, A11, B12, A13, B14, A15, B16, A17, B18, A19) => 2
sealed trait S
case object SA extends S
case object SB extends S
sealed trait U extends S
case object SC extends U
case object SD extends U
case class P(x: S, y: S)
def pairs(b: (Boolean, Boolean, Boolean)): Int = b match
  case (true, _, _) => 1
  case (_, true, _) => 2
def alternatives(p: P): Int = p match
  case P(SA | SB, SA) => 1
  case P(_: U, _) => 2
def nested(p: P): Int = p match
  case P(SA, _) => 1
  case P(_: U, SB) => 2
def twice(p: (S, S)): Int = p match
  case (SA, SA) => 1
  case (SB, SB) => 2
def deduped(x: (Boolean, Option[Boolean])): Int = x match
  case (true, Some(true)) => 1
  case (false, None) => 2
def guarded(s: S): Int = s match
  case x if x == SA => 1
def mixed(t: (S, Boolean)): Int = t match
  case (_: U, true) => 1
  case (SA, _) => 2
sealed trait K
case object KA extends K
case object KB extends K
case object KC extends K
def unreachable(x: (K, K, K)): Int = x match
  case (_, _, KA) => 0
  case (KB, _, _) => 1
  case (_, KC, KB) => 2
  case (_, _, KC) => 3
  case (KA, _, KC) => 4
// expect: exhaustivity_counterexamples.scala:70:25: error: match may not be exhaustive.
// expect: 10 errors found
