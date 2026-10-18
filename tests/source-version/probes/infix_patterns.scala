case class Cp(a: Int, b: Int)
infix case class Icp(a: Int, b: Int)
case class Bq(a: Int, b: Int)
object Ex { def unapply(x: Int): Option[(Int, Int)] = Some((x, x)) }
object Iex { infix def unapply(x: Int): Option[(Int, Int)] = Some((x, x)) }
object P {
  def f(x: AnyRef) = x match {
    case a Cp b => 1
    case a Icp b => 2
    case a `Bq` b => 4
    case _ => 0
  }
  def g(x: Int) = x match {
    case a Ex b => a + b
  }
  def h(x: Int) = x match {
    case a Iex b => a + b
  }
  def k(x: List[Int]) = x match { case h :: t => h; case Nil => 0 }
}
