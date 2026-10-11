// expect: 11:44: error: type mismatch: found Any, required String
// absent: refinement_intersection_overload.scala:7:
// Two parts of an intersection refining one method differently make an overload
// (`Types.findMember`'s `goAnd`, `Denotation.meet`), which no refinement narrows: the call has the
// class member's result, as scalac's (`Found: Any`); a part alone refining it narrows it.
trait Rec0[K <: String] { def get(k: K): Any }
type Rec[K <: String, V] = Rec0[K] { def get(k: K): V }
def one(r: Rec["k", String]): String = r.get("k")
// the intersection:
def r2: Rec["k", String] & Rec["v", Int] = ???
def res1: String = r2.get("k").toString + (r2.get("k"): String)
