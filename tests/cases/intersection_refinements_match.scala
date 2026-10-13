// Two parts refining one member with the same parameters: each refinement meets the member it
// refines (`goRefined`'s joint), and the two meet (`infoMeet`), the results intersected.
import scala.reflect.Selectable.reflectiveSelectable
trait R[K <: String] { def get(k: K): Any }
class RS extends R["k"] { def get(k: "k"): Product & Serializable = Some(k) }
type S = R["k"] { def get(k: "k"): Product }
type T = R["k"] { def get(k: "k"): Serializable }
type Rec[K <: String, V] = R[K] { def get(k: K): V }
def good(r: S & T): Product & Serializable = r.get("k")
def ints(r: Rec["k", Any] & Rec["k", Int]): Int = 0

@main def main(): Unit =
  println(good((new RS).asInstanceOf[S & T]))
