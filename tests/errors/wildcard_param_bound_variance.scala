// expect: 15:36: error: type mismatch: found C[_, _], required C[?, ? <: (Any => Int)]
// expect: 16:35: error: type mismatch: found D[_, _], required D[?, ? <: Contra[Any]]
// expect: 2 errors found
// A wildcard argument stands in its parameter's bound for the end its variance takes there
// (`TypeComparer.isSubArgs`' `paramBounds` through `substApprox`): in `F <: (A => Int)` the unknown
// `A` is contravariant, its lower end, so `F` is bounded by `Nothing => Int`, no `Any => Int`, and
// `C[?, ?]` is no `C[?, ? <: (Any => Int)]`, as scalac finds.
class C[A, F <: (A => Int)](val f: F)
trait Contra[-A]
class D[A, F <: Contra[A]](val n: Int)
class X extends Contra[String]

@main def run(): Unit =
  val c: C[?, ?] = new C[String, String => Int](_.length)
  val d: C[?, ? <: (Any => Int)] = c
  val e: D[?, ? <: Contra[Any]] = new D[String, X](12): D[?, ?]
  println(d.f(1) + e.n)
