// `CheckNonCyclicMap.checkInfo` (Checking.scala 303-304) checks a lower bound at variance -1, `nestedCycleOK` false:
// a lower bound naming its own parameter or member inside a type argument is an illegal cycle, scalac 3.8.4's E140
// "illegal cyclic type reference: lower bound .. refers back to the type itself", for a class's parameter, a type
// member and a method's parameter, whatever the argument's variance; the information is then the error type. A
// member's own reference through its owner's `this` is the cycle at once (`tp.symbol == sym`). The closure adversary
// of the pass on 97f0487ac (round 24): master rejected the class's and an F-bounded member's with a bound violation
// and accepted the rest, the round's narrowBound all. The member's F-bounded form is
// tests/pending-cases/bound_check_after_typer_error (teq's bound check runs after a typer error, scalac's does not).
trait BS[T, S <: BS[T, S]]
class Inv[T]
class Ct[-A]
class Q[S >: BS[?, S]]
trait H { type S >: Inv[S] }
trait H2 { type S >: List[S] }
class R[S >: Ct[Ct[S]] <: AnyRef]
def g[S >: Inv[S]]: Int = 1
def h[S >: List[S]]: Int = 1
// expect: lower_bound_cycle_illegal.scala:12:9: error: illegal cyclic type reference: lower bound BS[?, S] of type S refers back to the type itself
// expect: lower_bound_cycle_illegal.scala:13:16: error: illegal cyclic type reference: lower bound Inv[H.this.S] of type S refers back to the type itself
// expect: lower_bound_cycle_illegal.scala:14:17: error: illegal cyclic type reference: lower bound List[H2.this.S] of type S refers back to the type itself
// expect: lower_bound_cycle_illegal.scala:15:9: error: illegal cyclic type reference: lower bound Ct[Ct[S]] of type S refers back to the type itself
// expect: lower_bound_cycle_illegal.scala:16:7: error: illegal cyclic type reference: lower bound Inv[S] of type S refers back to the type itself
// expect: lower_bound_cycle_illegal.scala:17:7: error: illegal cyclic type reference: lower bound List[S] of type S refers back to the type itself
// expect: 6 errors found
