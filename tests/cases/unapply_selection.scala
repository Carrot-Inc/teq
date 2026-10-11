// The extractor of a pattern is chosen as dotty's `trySelectUnapply` chooses it: among overloaded
// `unapply` methods the most specific that takes the scrutinee's type, else the most specific of
// all; a case class's companion applies its own `unapply` where it takes the class (the
// synthetic one stands aside) or where overloading prefers it, and the fields otherwise.
object E:
  def unapply(x: String): Option[Int] = Some(1)
  def unapply(x: Int): Option[Int] = Some(2)
object F:
  def unapply(x: Any): Option[Int] = Some(3)
  def unapply(x: Int): Option[Int] = Some(4)
class B
class S1 extends B
class S2 extends S1
object K:
  def unapply(x: S1): Option[Int] = Some(11)
  def unapply(x: S2): Option[Int] = Some(12)

case class C(x: Int)
object C:
  def unapply(c: C): Option[Int] = Some(c.x + 10)
case class D(x: Int, y: String)
object D:
  def unapply(s: String): Option[(Int, String)] = Some((s.length, s))
case class P(x: Int)
object P:
  def unapply(p: P): Some[Int] = Some(p.x * 100)
sealed trait T
case class L(n: Int) extends T
object L:
  def unapply(l: L): Option[Int] = if l.n > 0 then Some(l.n) else None

@main def run(): Unit =
  println(7 match { case E(n) => n; case _ => 0 })
  println("s" match { case E(n) => n; case _ => 0 })
  println(7 match { case F(n) => n; case _ => 0 })
  println(("s": Any) match { case F(n) => n; case _ => 0 })
  println((new S2: B) match { case K(n) => n; case _ => 0 })
  println((new S1: B) match { case K(n) => n; case _ => 0 })
  println((C(1): Any) match { case C(n) => n; case _ => 0 })
  println(D(1, "a") match { case D(n, s) => s"$n$s" })
  println("abc" match { case D(n, s) => s"$n$s"; case _ => "" })
  println(P(3) match { case P(n) => n })
  val t: T = L(0)
  println(t match { case L(n) => n; case _ => -5 })
  println(List(C(2), C(3)).collect { case C(n) => n })
