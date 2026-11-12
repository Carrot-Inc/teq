// expect: 31:15: error: no given instance of type TC[Z, X] was found for parameter tc
// expect: 1 error found
// A divergence the goals' growth could hide: `g` is asked again for `TC[K2, B]` while it is open for `TC[Z, X]`, and
// the using clause between them (`Wrap[A, B]`) has by then given the open goal's variable a `Box`. The open goal is
// measured where it is opened (dotty keeps `typeSize` and `coveringSet` with the open search, `Implicits.OpenSearch`,
// since "types in search histories can contain type variables that can be instantiated by nested implicit
// searches"), so the request is of the same classes and larger, and is cut. scalac 3.8.4: E172, "No given instance
// of type TC[Z, X] was found for parameter tc of method need ... given instance g in object G produces a diverging
// implicit search when trying to match type TC[K2, B]".
trait TC[K, A]:
  def name: String
trait Step[K, K2]
trait Wrap[A, B]
final class Z
final class S[N]
final class Box[T]
final class Leaf
object G:
  given s0: Step[Z, S[Z]] = new Step[Z, S[Z]] {}
  given s1: Step[S[Z], S[S[Z]]] = new Step[S[Z], S[S[Z]]] {}
  given s2: Step[S[S[Z]], S[S[S[Z]]]] = new Step[S[S[Z]], S[S[S[Z]]]] {}
  given w[T]: Wrap[Box[T], T] = new Wrap[Box[T], T] {}
  given leaf: TC[S[S[S[Z]]], Leaf] = new TC[S[S[S[Z]]], Leaf] { def name = "leaf" }
  given g[K, K2, A, B](using st: Step[K, K2], wr: Wrap[A, B], tc: TC[K2, B]): TC[K, A] = new TC[K, A] { def name = "g " + tc.name }

import G.given

def need[X](using tc: TC[Z, X]): String = tc.name

@main def run(): Unit =
  println(need)
