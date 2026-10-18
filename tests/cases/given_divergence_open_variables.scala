// The divergence check under recursive givens whose requests hold the variable a using clause's `Out` stands for,
// each target measured as scalac measures it (`Implicits.checkDivergence` over `typeSize`, `coveringSet` and
// `wildApprox`): the variable counted once, as the larger of its two sides' bounds, each narrowed as the
// constraint holds it (one type where the instance equals `Out`, a lower bound alone where `Out` is only bounded).
// The swap's second request is as large as the first and unlike it, its third alike the second (both of an open
// variable), where `base` answers; growth over the same classes diverges at its second request; two variables
// bounded by one type; a bound naming its variable; a by-name knot; a nested ambiguity; a candidate that binds the
// variable and fails, rolled back for the next one.
trait Show[A] { def show: String }
final case class P[A, B](a: A, b: B)
final case class W[A](a: A)
def mk[A](s: String): Show[A] = new Show[A] { def show = s }

// Equal-size alternation: the swap keeps the size, so the second request for `rotated` is no smaller and unlike
// the first (no divergence), the third alike the first (divergence), where `base` answers.
object Swap:
  trait Rot[T] { type Out }
  object Rot:
    type Aux[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Aux[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T, O](using r: Rot.Aux[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String =
    summon[Show[W[P[Int, Long]]]].show

// Growth: each request for `grow` is for a larger type made of the same classes, so the second diverges.
object Grow:
  trait Wrap[T] { type Out }
  object Wrap:
    type Aux[T, O] = Wrap[T] { type Out = O }
    given opt[T]: Aux[T, Option[T]] = new Wrap[T] { type Out = Option[T] }
  trait LowShow:
    given base[A]: Show[A] = mk("base")
  object Instances extends LowShow:
    given grow[T, O](using w: Wrap.Aux[T, O], s: Show[O]): Show[T] = mk("grow(" + s.show + ")")
  def run(): String =
    import Instances.given
    summon[Show[Int]].show

// Shared bounds: two open variables bounded by one type, both in the next request.
object Shared:
  trait Dup[T] { type L; type R }
  object Dup:
    type Aux[T, A, B] = Dup[T] { type L = A; type R = B }
    given dup[T]: Aux[T, T, T] = new Dup[T] { type L = T; type R = T }
  given pair[T, A, B](using d: Dup.Aux[T, A, B], s: Show[P[A, B]]): Show[W[T]] = mk("pair(" + s.show + ")")
  given ps[A, B](using a: Show[A], b: Show[B]): Show[P[A, B]] = mk("ps(" + a.show + ", " + b.show + ")")
  given wrapped[T, A, B](using d: Dup.Aux[T, A, B], s: Show[P[W[A], B]]): Show[W[W[T]]] = mk("wrapped(" + s.show + ")")
  given ints: Show[Int] = mk("int")
  def run(): String =
    summon[Show[W[Int]]].show + " " + summon[Show[W[W[Int]]]].show

// A cyclic bound: the variable's upper bound names it (`A <: Ranked[A]`).
object Cyclic:
  trait Ranked[A <: Ranked[A]]
  final case class Gold() extends Ranked[Gold]
  given gold: Show[Gold] = mk("gold")
  given nest[T](using s: Show[T]): Show[W[T]] = mk("nest(" + s.show + ")")
  def best[A <: Ranked[A]](using s: Show[W[A]]): String = s.show
  def run(): String = best

// A by-name knot through an open variable: `Show[Tree]` needs `Show[List[Tree]]` by name, which needs
// `Show[Tree]` again, answered by the instance under construction.
object Knot:
  final case class Tree(children: List[Tree])
  trait Norm[T] { type Out }
  object Norm:
    type Aux[T, O] = Norm[T] { type Out = O }
    given same[T]: Aux[T, T] = new Norm[T] { type Out = T }
  given showTree(using s: => Show[List[Tree]]): Show[Tree] with
    def show = "tree"
  given showList[A](using s: Show[A]): Show[List[A]] = mk("list(" + s.show + ")")
  def render[T, O](using n: Norm.Aux[T, O], s: Show[List[O]]): String = s.show
  def run(): String = render[Tree, Tree]

// A nested ambiguity: `first` fails on its ambiguous `M`, `exact` answers.
object Ambiguous:
  trait M[A]
  object M:
    given a[A]: M[A] = new M[A] {}
    given b[A]: M[A] = new M[A] {}
  trait Norm[T] { type Out }
  object Norm:
    type Aux[T, O] = Norm[T] { type Out = O }
    given same[T]: Aux[T, T] = new Norm[T] { type Out = T }
  trait C[A] { def name: String }
  object C:
    given first[A](using M[A]): C[W[A]] = new C[W[A]] { def name = "first" }
    given exact: C[W[Int]] = new C[W[Int]] { def name = "exact" }
  def pick[T, O](using n: Norm.Aux[T, O], c: C[W[O]]): String = c.name
  def run(): String = pick[Int, Int]

// A failed attempt rolled back: `narrow` binds the open variable to `Long` and fails on its using clause, the
// next candidate sees the variable as it was.
object Rollback:
  trait Norm[T] { type Out }
  object Norm:
    type Aux[T, O] = Norm[T] { type Out = O }
    given same[T]: Aux[T, T] = new Norm[T] { type Out = T }
  trait Missing[A]
  given narrow(using Missing[Long]): Show[W[Long]] = mk("narrow")
  given general[T](using s: Show[T]): Show[W[T]] = mk("general(" + s.show + ")")
  given ints: Show[Int] = mk("int")
  def pick[T, O](using n: Norm.Aux[T, O], s: Show[W[O]]): String = s.show
  def run(): String = pick[Int, Int]

// The swap and the growth with `Out` bounded above, so that each request holds an open variable bounded below.
object SwapOpen:
  trait Rot[T] { type Out }
  object Rot:
    type Below[T, O] = Rot[T] { type Out <: O }
    type Exact[T, O] = Rot[T] { type Out = O }
    given swap[A, B]: Exact[P[A, B], P[B, A]] = new Rot[P[A, B]] { type Out = P[B, A] }
  given rotated[T, O](using r: Rot.Below[T, O], s: Show[W[O]]): Show[W[T]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show

object GrowOpen:
  trait Wrap[T] { type Out }
  object Wrap:
    type Below[T, O] = Wrap[T] { type Out <: O }
    type Exact[T, O] = Wrap[T] { type Out = O }
    given opt[T]: Exact[T, Option[T]] = new Wrap[T] { type Out = Option[T] }
  trait LowShow:
    given base[A]: Show[A] = mk("base")
  object Instances extends LowShow:
    given grow[T, O](using w: Wrap.Below[T, O], s: Show[O]): Show[T] = mk("grow(" + s.show + ")")
  def run(): String =
    import Instances.given
    summon[Show[Int]].show

@main def run(): Unit =
  println(Swap.run())
  println(Grow.run())
  println(Shared.run())
  println(Cyclic.run())
  println(Knot.run())
  println(Ambiguous.run())
  println(Rollback.run())
  println(SwapOpen.run())
  println(GrowOpen.run())
