// A request for a given already open for a type of its size and classes that is unlike it does not decide the
// divergence check: the check goes on to the requests opened before (`Implicits.checkDivergence`'s
// `loop(outer, ..)`), so a swap diverges at its third request, alike its first, and a rotation of three at its
// fourth, where `base` answers, rather than recurring to the depth limit.
trait Show[A] { def show: String }
final case class P[A, B](a: A, b: B)
final case class Q[A, B, C](a: A, b: B, c: C)
final case class W[A](a: A)
def mk[A](s: String): Show[A] = new Show[A] { def show = s }

object Swap:
  given rotated[A, B](using s: Show[W[P[B, A]]]): Show[W[P[A, B]]] = mk("rotated(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[P[Int, Long]]]].show

object Cycle:
  given turned[A, B, C](using s: Show[W[Q[B, C, A]]]): Show[W[Q[A, B, C]]] = mk("turned(" + s.show + ")")
  given base[A]: Show[A] = mk("base")
  def run(): String = summon[Show[W[Q[Int, Long, String]]]].show

@main def run(): Unit =
  println(Swap.run())
  println(Cycle.run())
