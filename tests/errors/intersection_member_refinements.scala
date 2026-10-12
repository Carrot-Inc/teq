// expect: 28:14: error: Ambiguous overload. The overloaded alternatives of method (k: "k"): String with types
// expect:  (k: "k"): String
// expect:  (k: "v"): Int
// expect: both match arguments ("v")
// expect: Note: Overloaded definitions introduced by refinements cannot be resolved
// expect: 29:14: error: None of the overloaded alternatives of method get in trait R with types
// expect:  (k: "v" | "k"): Any
// expect:  (k: "v"): Int
// expect:  (k: "k"): String
// expect: match arguments ("k")
// expect: 30:17: error: Ambiguous overload. The overloaded alternatives of method (k: "v"): Int with types
// expect:  (k: "v"): Int
// expect:  (k: "k"): String
// expect: both match arguments ("k")
// absent: intersection_member_refinements.scala:32:
// expect: 3 errors found
// Refinements of one member on an intersection's parts, each beside the member seen from the whole
// receiver (`goRefined`'s meet, `Denotation.meet`'s `MultiDenotation` where the parameters do not
// match): the alternatives without a symbol are one reference, the last refinement's
// (`designatorFor`), so `"v"` is ambiguous between two refinements however the call's result is
// expected, and in the other order, where the shared reference is `"k"`'s, `"k"` is ambiguous where
// `String` is expected and none applies where `Int` is (`resolveOverloaded1`'s retry with the
// expected type, `deepPt`). One refinement beside the member is chosen where it applies.
import scala.reflect.Selectable.reflectiveSelectable
trait R[K <: String] { def get(k: K): Any }
type Rec[K <: String, V] = R[K] { def get(k: K): V }
def r: Rec["k", String] & Rec["v", Int] = ???
def a: Int = r.get("v")
def b: Int = (r: Rec["v", Int] & Rec["k", String]).get("k")
def c: String = (r: Rec["v", Int] & Rec["k", String]).get("k")
def one: Rec["k", String] & R["v"] = ???
def d: String = one.get("k")
