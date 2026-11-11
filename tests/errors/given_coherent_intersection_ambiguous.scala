// expect: 15:44: error: ambiguous given instances for CanEqual[A, B] & Marker: a, b
// expect: 16:44: error: ambiguous given instances for Marker & CanEqual[A, B]: a, b
// expect: 2 errors found
// `isCoherent` is `pt.isRef(defn.CanEqualClass)` (Implicits.scala:1280), and `Type.isRef` (Types.scala:200-219) reads
// no part of an intersection: a search for `CanEqual[A, B] & Marker` is no CanEqual search, so `rank` ranks its two
// givens and finds them alike, in either order of the intersection (scalac 3.8.4: "Ambiguous given instances: both
// given instance a in object O and given instance b in object O match type CanEqual[A, B] & Marker", and the same for
// the reversed type).
class A
class B
trait Marker
object O:
  given a: CanEqual[A, B] & Marker = ???
  given b: CanEqual[A, B] & Marker = ???
  def one = summon[CanEqual[A, B] & Marker]
  def two = summon[Marker & CanEqual[A, B]]
