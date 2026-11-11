// The parameters of an abstract type member applied (`checkAppliedType`'s `typeParams`, Checking.scala 116-118) have
// the bounds of the member's `TypeRef`'s info, seen from the prefix: the declaring class's parameters, and its
// enclosing classes', are the prefix's arguments there (scala3's tests/pos/i4884.scala and i4906.scala), so
// `type M[X <: U]` through a `T[Any]` takes `String` and through a `T[Int]` does not. A higher-kinded argument of a
// parameter whose bound is its kind's top (`M[_[_]]`'s `[_] =>> Any`) is within it, a refinement's own member too
// (tests/pos/i7602.scala). scalac 3.8.4 rejects the two lines (E057):
//   15:22 Type argument String does not conform to upper bound Int
//   17:12 Type argument String does not conform to upper bound Int
// and accepts the rest.
trait T[U]:
  type M[X <: U]
def f(t: T[Any]): t.M[String] = ???
class S extends T[Any]:
  def h: M[String] = ???
def g(t: T[Int]): t.M[String] = ???
class S2 extends T[Int]:
  def h2: M[String] = ???
trait O[D]:
  type F[_ <: D]
  trait I[E]:
    type G[_ <: D & E]
trait A
trait B
def k(o: O[A], i: o.I[B]): Unit =
  type P[a <: A] = o.F[a]
  type Q[c <: A & B] = i.G[c]
trait Monad[F[_]]
type Of[M[_[_]], X] = ({ type F[_]; type R = M[F] ?=> F[X] })#R
def m(a: Of[Monad, String]) = ???
// expect: 15:23: error: type argument String does not conform to upper bound Int
// expect: 17:13: error: type argument String does not conform to upper bound Int
// expect: 2 errors found
