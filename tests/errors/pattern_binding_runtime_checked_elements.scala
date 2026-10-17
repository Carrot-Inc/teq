// teq: --werror
// A component's annotation is read as dotty reads it (`Checking.checkIrrefutable`'s `recur`): on the
// component type `UnapplyArgs` gives over the extractor as instantiated, whose inference records an
// annotated argument's underlying type (`List.unapplySeq[Option[Int]]` over a `List[Option[Int]
// @RuntimeChecked]`, a sequence's, an array's and a generic case class's alike), and on the type as
// named, not dealiased (Types.scala 438-440): an alias of an annotated type exempts nothing.
import scala.annotation.internal.RuntimeChecked
type Checked = Option[Int] @RuntimeChecked
object Aliased:
  def unapply(x: Int): Tuple1[Checked] = Tuple1(Some(x))
val xs: List[Option[Int] @RuntimeChecked] = List(Some(1))
val ys: Seq[Option[Int] @RuntimeChecked] = Seq(Some(1))
val zs: Array[Option[Int] @RuntimeChecked] = Array(Some(1))
case class Holder[A](x: A)
val holder: Holder[Option[Int] @RuntimeChecked] = Holder(Some(1))
object Members:
  val List(Some(m)) = xs
def locals(): Unit =
  val List(Some(a)) = xs
  val Seq(Some(b)) = ys
  val Array(Some(c)) = zs
  val Aliased(Some(d)) = 7
  val Holder(Some(e)) = holder
  ()
// expect: 17:23: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 19:23: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 20:22: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 21:24: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 22:26: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: 23:25: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
