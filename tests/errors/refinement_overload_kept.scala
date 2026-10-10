// expect: refinement_overload_kept.scala:9:
// Two parts' refinements of one method that differ make an overload, which stays one when a third
// part refines the method too (`Denotation.meet` of a `MultiDenotation`): no refinement narrows the
// call, which scalac finds ambiguous (`neg/i8736`).
trait Rec0[K <: String] { def get(k: K): Any }
type Rec[K <: String, V] = Rec0[K] { def get(k: K): V }
def r3: Rec["k", String] & Rec["v", Int] & Rec["z", Boolean] = ???
// the third part's refinement alone would type this:
def res3: Boolean = r3.get("z")
