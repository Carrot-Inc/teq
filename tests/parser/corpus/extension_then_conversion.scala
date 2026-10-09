// An extension named without arguments whose application fails, or whose result does not fit
// the expected type, gives way to an implicit conversion that has the member (dotc's
// `tryExtensionOrConversion`): `props.toTagMod` against a `TagMod` cannot eta-expand the
// extension into the trait, whose method takes a `Builder`, so `Ext(props).toTagMod` with
// `$conforms` is meant; `Seq(1, 2).total` against a `String` is the conversion's.
class Builder
trait TagMod:
  def applyTo(b: Builder): Unit
object Lib:
  implicit final class Ext[A](private val as: Seq[A]) extends AnyVal:
    def toTagMod(implicit f: A => TagMod): TagMod = new TagMod:
      def applyTo(b: Builder): Unit = as.foreach(a => f(a).applyTo(b))
  implicit class Totals(s: Seq[Int]):
    def total: String = "conversion " + s.sum
object Prelude:
  extension [A](as: Seq[A])
    def toTagMod(f: A => TagMod): TagMod = Lib.Ext(as).toTagMod(using f)
  extension (s: Seq[Int]) def total: Int = s.sum
import Lib.*
import Prelude.*
class Leaf(n: Int) extends TagMod:
  def applyTo(b: Builder): Unit = println(s"leaf $n")
object Main:
  def render(props: Seq[TagMod]): TagMod = props.toTagMod
  def main(args: Array[String]): Unit =
    render(Seq(Leaf(1), Leaf(2))).applyTo(new Builder)
    Seq(Leaf(3)).toTagMod(l => l).applyTo(new Builder)
    val t: String = Seq(1, 2).total
    println(t)
    val u: Int = Seq(1, 2).total
    println(u)
    val w: Long = Seq(1, 2).total
    println(w)
