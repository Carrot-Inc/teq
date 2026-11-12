// A given asks `Wrap[A, B]` and then that there be no `TC[B]`: the search for `TC[B]` under it is of the open
// goal's classes as it was opened and larger, so it is cut and `NotGiven` holds. Measured at the check, after `Wrap`
// has given the open goal's variable a `Box`, it would not be cut and the depth of the search would decide.
// scalac 3.8.4 prints `1`.
import scala.util.NotGiven
trait TC[A]
trait Wrap[A, B]
final class Box[T]
object G:
  given w[T]: Wrap[Box[T], T] = new Wrap[Box[T], T] {}
  given g[A, B](using Wrap[A, B], NotGiven[TC[B]]): TC[A] = new TC[A] {}

import G.given

def need[X](using tc: TC[X]): Int = 1

@main def run(): Unit =
  println(need)
