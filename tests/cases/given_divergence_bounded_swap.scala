// The swap's recursion where the variable `O` is bounded as well (`O <: P[?, ?]`, `<: Product`, `<: AnyRef`, or `T`
// bounded): its bounds are measured as dotty narrows them when each is added (`ConstraintHandling.addOneBound`: `lo |
// bound`, `hi & bound`, a part another covers dropped), so that `O >: Nothing | P[Long, Int] <: P[?, ?] &
// P[Long, Int]` counts as `P[Long, Int]` once, not as each of its bounds, and the swap's third request, alike its
// second (both of an open variable), diverges where `base` answers. Where `Out` is only bounded the variable is
// measured the same way.
trait Show[A] { def show: String }
final case class P[A, B](a: A, b: B)
final case class W[A](a: A)
def mk[A](s: String): Show[A] = new Show[A] { def show = s }
object V0:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T, O](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show
object V1:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T, O <: Product](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show
object V2:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T, O <: AnyRef](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show
object V3:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T, O <: P[?, ?]](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show
object V4:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T <: Product, O](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show
object V5:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T <: Product, O <: Product](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show
object Open:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    type Below[T, O] = Rot[T] { type Out <: O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T, O <: P[?, ?]](using r: Rot.Below[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show
@main def run(): Unit =
  println("V0: " + V0.run())
  println("V1: " + V1.run())
  println("V2: " + V2.run())
  println("V3: " + V3.run())
  println("V4: " + V4.run())
  println("V5: " + V5.run())
  println("Open: " + Open.run())
