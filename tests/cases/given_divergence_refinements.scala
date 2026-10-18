// Recursive givens whose requests are refinements over open variables. The size visits a refinement's parent
// before its member (`Types.TypeSizeAccumulator`'s `foldOver`), each variable once a walk: `Box[V] { type Out = U }`
// with `V <: List[List[Int]]` and `U >: L[V] <: Upper[List[Int]]` measures 7 with its `Show`, larger than the first
// request's 6, so it diverges where `base` answers (the member first, `V` met inside `U`'s bound, measured 6 and the
// recursion went on). The wildcard maps the member too and makes each variable a wildcard of its bounds
// (`ProtoTypes.wildApprox`), compared as `=:=` compares them: two requests `Box[V] { type Out = U }` of fresh
// variables are alike, so the second diverges; a sum over unary numbers, `Sum[M, Succ[N]] { type Out = R }` with `M`
// one smaller at each request, is unlike the request before, its wildcards' bounds apart, and goes to its end.
trait Show[A] { def show: String }
class Upper[A]
class L[A] extends Upper[List[Int]]
class Box[A]
def mk[A](s: String): Show[A] = new Show[A] { def show = s }
trait LowShow:
  given base[A]: Show[A] = mk("base")

object Order extends LowShow:
  given nest[A, B, V <: List[List[Int]], U >: L[V] <: Upper[List[Int]]](using s: Show[Box[V] { type Out = U }]): Show[Box[A] { type Out = B }] =
    mk("nest(" + s.show + ")")
  def run(): String = summon[Show[Box[L[Int]] { type Out = Upper[List[Int]] }]].show

object Alike extends LowShow:
  given nest[A, B, V, U](using s: Show[Box[V] { type Out = U }]): Show[Box[A] { type Out = B }] = mk("nest(" + s.show + ")")
  def run(): String = summon[Show[Box[Int] { type Out = Int }]].show

object Sums:
  sealed trait Nat
  sealed trait Succ[Prev <: Nat] extends Nat
  sealed trait Zero extends Nat
  class Sum[M <: Nat, N <: Nat](val steps: Int) { type Out <: Nat }
  object Sum:
    type Aux[M <: Nat, N <: Nat, R <: Nat] = Sum[M, N] { type Out = R }
    given done[N <: Nat]: Aux[Zero, N, N] = new Sum[Zero, N](0) { type Out = N }
    given step[M <: Nat, N <: Nat, R <: Nat](using s: Aux[M, Succ[N], R]): Aux[Succ[M], N, R] =
      new Sum[Succ[M], N](s.steps + 1) { type Out = R }
  type Three = Succ[Succ[Succ[Zero]]]
  type Five = Succ[Succ[Three]]
  def run(): String = "sum after " + summon[Sum[Three, Five]].steps + " steps"

@main def run(): Unit =
  println(Order.run())
  println(Alike.run())
  println(Sums.run())
