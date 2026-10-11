// expect: 13:44: error: type mismatch: found Any, required String
// absent: intersection_member_refined.scala:11:
// expect: 1 error found
// Two parts refining the member differently: neither refinement's parameter matches the member
// seen from the whole receiver (`"k" | "v"`), so each stays an alternative beside it
// (`Denotation.meet`), and the call of `"k"` is the class member's, of result `Any`: the argument
// is taken, the result refused. A part alone refining the member narrows it.
import scala.reflect.Selectable.reflectiveSelectable
trait Rec0[K <: String] { def get(k: K): Any }
type Rec[K <: String, V] = Rec0[K] { def get(k: K): V }
def one(r: Rec["k", String]): String = r.get("k")
def r2: Rec["k", String] & Rec["v", Int] = ???
def res1: String = r2.get("k").toString + (r2.get("k"): String)
