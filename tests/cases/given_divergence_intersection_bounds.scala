// A variable bounded above by two intersections that share an operand (`V <: P[Int] & Q[Long]` declared, and
// `P[Int] & R[Long]` from a contravariant `Bound[V]`): its bound is met as dotty meets it (`hi & bound`,
// `TypeComparer.glb` dropping each operand the other covers, `dropIfSuper`), `Q[Long] & P[Int] & R[Long]`, so
// `Show[W[V]]` measures 5 and is unlike the first request rather than larger than it; written with `R[Long]` in the
// declared bound the measure is the same, and both recur once more, where `base` answers, as in scalac.
trait Show[A] { def show: String }
class P[A]
class Q[A]
class R[A]
class W[A]
trait Bound[-A]
given Bound[P[Int] & R[Long]] = new Bound[P[Int] & R[Long]] {}
def mk[A](s: String): Show[A] = new Show[A] { def show = s }
trait Low:
  given base[A]: Show[A] = mk("base")
object Overlapping extends Low:
  given nest[T, V <: P[Int] & Q[Long]](using b: Bound[V], s: Show[W[V]]): Show[W[T]] = mk("nest(" + s.show + ")")
  def run(): String = summon[Show[W[P[Long] & Q[Int] & R[Int]]]].show
object Normalized extends Low:
  given nest[T, V <: P[Int] & Q[Long] & R[Long]](using b: Bound[V], s: Show[W[V]]): Show[W[T]] = mk("nest(" + s.show + ")")
  def run(): String = summon[Show[W[P[Long] & Q[Int] & R[Int]]]].show
@main def run(): Unit =
  println(Overlapping.run())
  println(Normalized.run())
