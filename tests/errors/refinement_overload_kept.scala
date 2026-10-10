// expect: 12:17: error: Ambiguous overload. The overloaded alternatives of method
// expect: Note: Overloaded definitions introduced by refinements cannot be resolved
// Two parts' refinements of one method that differ make an overload, which stays one when a third
// part refines the method too (`Denotation.meet` of a `MultiDenotation`): no refinement narrows the
// call, and the call does not fall back to the member's info. Arguments that fit the rightmost part's
// refinement are ambiguous (`neg/i8736`): dotty's alternatives share the member's `TermRef`, which
// holds the last one (`designatorFor`, `withDenot`).
trait Rec0[K <: String] { def get(k: K): Any }
type Rec[K <: String, V] = Rec0[K] { def get(k: K): V }
def r3: Rec["k", String] & Rec["v", Int] & Rec["z", Boolean] = ???
// the class member alone would type this, and the third part's refinement too:
def res3: Any = r3.get("z")
// two parts, the rightmost one's argument, in either order:
def r2: Rec["k", String] & Rec["v", Int] = ???
def res2: Any = r2.get("v")
def rv: Rec["v", Int] & Rec["k", String] = ???
def resv: Any = rv.get("k")
// expect: 15:17: error: Ambiguous overload.
// expect: 17:17: error: Ambiguous overload.
