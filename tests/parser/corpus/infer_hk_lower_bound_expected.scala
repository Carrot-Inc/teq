// A higher-kinded variable with a lower bound (`G[x] >: Cod[x]` of skunk's `*:`) applied to an
// argument and checked against an expected class above the bound: the application takes the
// expected class, whose bound is then checked, instead of the bound itself.
class Dec[A](val s: String)
class Cod[A](s: String) extends Dec[A](s)
final class Ops[F[_], B](self: F[B]):
  def pre[G[x] >: F[x], A](ga: G[A]): G[(A, B)] = ga.asInstanceOf[G[(A, B)]]
def pre2[G[x] >: Cod[x], A](ga: G[A]): G[(A, Int)] = ga.asInstanceOf[G[(A, Int)]]
@main def main(): Unit =
  val o = new Ops(new Cod[Int]("c"))
  val t: Dec[(String, Int)] = o.pre(new Dec[String]("d"))
  val u: Dec[(String, Int)] = pre2(new Dec[String]("e"))
  println(t.s + u.s)
