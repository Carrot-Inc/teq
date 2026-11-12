// expect: 18:21: error: type argument Int does not have the same kind as its bound [_]
// expect: 19:19: error: type argument List does not have the same kind as its bound
// expect: 21:20: error: type argument Int does not have the same kind as its bound [_]
// expect: 24:12: error: type argument Int does not have the same kind as its bound [_]
// expect: 26:13: error: type argument ? >: List does not have the same kind as its bound [_]
// expect: 27:13: error: type argument ? <: Int does not have the same kind as its bound [_]
// expect: 30:13: error: type argument Int does not have the same kind as its bound [_]
// expect: 35:13: error: type argument ? <: List does not have the same kind as its bound
// expect: 37:14: error: type argument ? <: List does not have the same kind as its bound
// expect: 9 errors found
// `preCheckKinds` (Typer.scala 2794-2802, Checking.scala 189-200) asks every argument of every application whose
// constructor takes type parameters, whatever the constructor: a class, a lambda, a higher-kinded parameter (`G` of
// `G[F[_]]`, `G` of `G[_]`), an abstract type member (`h.M`, `N`), in a pattern too; its one exemption the unbounded
// wildcard (`WildcardTypeBoundsTree`, `NoType` bounds), a bounded
// one asked as its `TypeBounds`, whose kind is its upper bound's
// (`? >: List` a proper type's, `? <: List` a constructor's).
// scalac 3.8.4 rejects each line marked so and accepts the others.
def f[G[F[_]]](x: G[Int]): Unit = ()
def f2[G[_]](x: G[List]): Unit = ()
trait H { type M[F[_]] }
def g(h: H)(x: h.M[Int]): Unit = ()
trait K:
  type N[F[_]]
  def y: N[Int]
class C[F[_]]
type W1 = C[? >: List]
type W2 = C[? <: Int]
type W3 = C[? <: List]
def p(x: Any) = x match
  case _: C[Int] => 1
  case _ => 2
def ok[G[F[_]]](x: G[List], y: G[Nothing], z: G[?]): Unit = ()
def okm(h: H)(x: h.M[List], y: h.M[? <: List]): Unit = ()
class D[A <: Int]
type W4 = D[? <: List]
class Co[+A]
type W5 = Co[? <: List]
