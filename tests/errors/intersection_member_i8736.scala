// expect: 20:17: error: type mismatch: found Any, required String
// expect: 21:14: error: type mismatch: found Any, required Int
// expect: 22:18: error: Ambiguous overload. The overloaded alternatives of method (k: "k"): String with types
// expect:  (k: "k"): String
// expect:  (k: "v"): Int
// expect:  (k: "z"): Boolean
// expect: all match arguments (String)
// expect: Note: Overloaded definitions introduced by refinements cannot be resolved
// expect: 3 errors found
// scala3's neg/i8736 reduced: three parts refining one member with three parameters. The member
// seen from the whole receiver takes `"k" | "v" | "z"` and returns `Any`; each refinement stays an
// alternative beside it, and the alternatives without a symbol are one reference (dotty's
// `designatorFor` names them by their name alone), the last refinement's, so the calls of `"k"`
// and `"v"` are the class member's, of result `Any`, and that of `"z"` is ambiguous among the
// three refinements.
import scala.reflect.Selectable.reflectiveSelectable
trait R[K <: String] { def get(k: K): Any }
type Rec[K <: String, V] = R[K] { def get(k: K): V }
def rec: Rec["k", String] & Rec["v", Int] & Rec["z", Boolean] = ???
def a: String = rec.get("k")
def b: Int = rec.get("v")
def c: Boolean = rec.get("z")
