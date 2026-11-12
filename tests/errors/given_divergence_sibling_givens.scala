// expect: 23:15: error: no given instance of type TC[X] was found for parameter tc
// expect: 1 error found
// Two givens of one signature but for a third clause, each asking `Wrap[A, B]` and then `TC[B]`: the request for
// `TC[B]` under either is of the classes the open goal had when it was opened and larger, and is cut at once, as
// scalac cuts it (3.8.4: E172, "given instance g1 in object G produces a diverging implicit search when trying to
// match type TC[B]"); measured at the check, after `Wrap` has given the open goal's variable a `Box`, the two would
// differ and the search would end by its budget alone.
trait TC[A]
trait Wrap[A, B]
final class Box[T]
trait Extra
object G:
  given extra: Extra = new Extra {}
  given w[T]: Wrap[Box[T], T] = new Wrap[Box[T], T] {}
  given g1[A, B](using Wrap[A, B], TC[B]): TC[A] = new TC[A] {}
  given g2[A, B](using Wrap[A, B], TC[B], Extra): TC[A] = new TC[A] {}

import G.given

def need[X](using tc: TC[X]): Int = 1

@main def run(): Unit =
  println(need)
