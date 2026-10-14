import scala.language.implicitConversions
// A conversion an imported object inherits from a generic trait is seen from that object: its
// result, a class nested in the trait, takes the object's arguments (`all.SeqNode[A]` of a
// `Util[Builder]` is a `Modifier[Builder]`), as scalatags' `div(Option(cls := "active"))` needs.
trait Modifier[B]:
  def show: String
case class AttrPair[B, T](v: T) extends Modifier[B]:
  def show = s"attr($v)"
trait LowPriUtil[B, O <: F, F]:
  implicit def OptionFrag[A](xs: Option[A])(implicit ev: A => Modifier[B]): Modifier[B] = new Modifier[B]:
    def show = "frag"
trait Util[B, O <: F, F] extends LowPriUtil[B, O, F]:
  implicit def OptionNode[A](xs: Option[A])(implicit ev: A => Modifier[B]): SeqNode[A] = new SeqNode(xs.toSeq)
  implicit class SeqNode[A](xs: Seq[A])(implicit ev: A => Modifier[B]) extends Modifier[B]:
    def show = xs.map(a => ev(a).show).mkString("seq(", ",", ")")
class Builder
object all extends Util[Builder, String, String]
import all.*

def div(ms: Modifier[Builder]*): String = ms.map(_.show).mkString("div(", ",", ")")

@main def run(): Unit =
  println(div(Option(AttrPair[Builder, String]("active"))))
  println(div(Option.empty[AttrPair[Builder, Int]], Seq(AttrPair[Builder, Int](1), AttrPair[Builder, Int](2))))
  val m: Modifier[Builder] = Option(AttrPair[Builder, String]("x"))
  println(m.show)
