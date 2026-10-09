// A case whose pattern is broken: the match's coverage over it is unknown, the other matches'
// is checked.
sealed trait T
case class A(x: Int) extends T
case class B(y: String) extends T
object O:
  def f(t: T): Int = t match
    case A(x) => x
    case B( => 2
  def g(t: T): Int = t match
    case => 1
    case A(x) => x
  def h(t: T): Int = t match
    case A(x) => x
  val bad: String = 3
