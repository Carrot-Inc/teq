// expect: 12:81: error: type mismatch: found List[Int | String], required Nothing & Any
// expect: 14:63: error: type mismatch: found [T <: Int | String] => (x$1: T) => T, required [T <: Int & String] => (x$1: T) => T
// expect: 16:42: error: type mismatch: found List[Int] & Option[Int], required Boolean
// expect: 3 errors found
// The last fold's refusals, as scalac 3.8.4 refuses them: a constructor range's lambda (a union
// of two classes' constructors, invariant) leaves a range argument nothing to fit where it is
// written (scalac's `Nothing & Any`); a polymorphic member's bound read as the union of the
// instances' arguments is no function of the intersection's bound (`comparePoly`'s erased
// parameters); and a constructor range applied is the intersection of the two classes'
// applications, which no `Boolean` is.
trait H[F[_], K] { def put(x: F[K]): Unit }
def f(h: H[List, Int] & H[Option, String])(x: List[Int | String]): Unit = h.put(x)
trait P[K] { def get: [T <: K] => T => T }
def g(p: P[Int] & P[String]): [T <: Int & String] => T => T = p.get
trait G[F[_]] { def get: F[Int] }
def e(g: G[List] & G[Option]): Boolean = g.get
