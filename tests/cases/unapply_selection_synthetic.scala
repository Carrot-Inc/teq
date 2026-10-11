// A case class's synthetic `unapply[Ts](x$1: C[Ts]): C[Ts]` is an ordinary candidate beside its
// companion's own, dropped only where one of those, declared or inherited, has its signature
// (`Namer.invalidateIfClashingSynthetic`); the whole set is resolved against the scrutinee, then
// a wildcard (`trySelectUnapply`), the fields read only where the synthetic one wins.
case class G[A](x: A)
object G:
  def unapply(g: G[String]): Option[Int] = Some(g.x.length)
trait L
trait R
case class Both(x: Int) extends L, R
object Both:
  def unapply(l: L): Option[Int] = Some(10)
  def unapply(r: R): Option[Int] = Some(20)
case class Inh(x: Int)
trait Extract:
  def unapply(c: Inh): Option[Int] = Some(c.x + 10)
object Inh extends Extract
case class Multi(x: Int)
object Multi:
  def unapply(x: String): Option[Int] = Some(10)
  def unapply(x: Int): Option[Int] = Some(20)
case class Gen[A](x: A)
object Gen:
  def unapply[B](g: Gen[B]): Option[String] = Some(s"own ${g.x}")

@main def run(): Unit =
  println((G(7): @unchecked) match { case G(n) => n; case _ => -1 })
  println(G("abcd") match { case G(n) => n; case _ => -1 })
  println((G("abc"): Any) match { case G(n) => n; case _ => -1 })
  println((Both(7): @unchecked) match { case Both(n) => n; case _ => -1 })
  println(Inh(2) match { case Inh(n) => n; case _ => -1 })
  println((Multi(3): @unchecked) match { case Multi(n) => n; case _ => -1 })
  println("s" match { case Multi(n) => n; case _ => -1 })
  println((Gen(5): @unchecked) match { case Gen(s) => s })
