// expect: 14:33: error: None of the overloaded alternatives of method unapply in object E with types
// expect: 15:36: error: None of the overloaded alternatives of method unapply in object D with types
// expect:  (s: String): Option[(Int, String)]
// expect:  (x$1: D): D
// expect: 2 errors found

object E:
  def unapply(x: String): Option[Int] = Some(1)
  def unapply(x: Int): Option[Int] = Some(2)
case class D(x: Int, y: String)
object D:
  def unapply(s: String): Option[(Int, String)] = Some((s.length, s))
@main def run(): Unit =
  println((7: Any) match { case E(n) => n; case _ => 0 })
  println((true: Any) match { case D(n, s) => s; case _ => "" })
