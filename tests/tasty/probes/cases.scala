// What PostTyper adds to case classes, and what a user's definition suppresses.
package probe.cases

case class Plain(a: Int, b: String = "b")
case class Custom(a: Int):
  override def toString: String = "custom"
  override def equals(o: Any): Boolean = false
  override def hashCode: Int = 1
  def copy(a: Int): Custom = Custom(a)
case class WithApply(a: Int)
object WithApply:
  def apply(a: Int, b: Int): WithApply = WithApply(a + b)
  def unapply(w: WithApply): Some[Int] = Some(w.a)
case class Poly[A](value: A, rest: List[A])
case object Single
case class Curried(a: Int)(val b: Int)
case class Vararg(xs: Int*)
final case class Private private (a: Int)
