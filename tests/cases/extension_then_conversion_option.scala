// An extension named without arguments whose eta-expansion cannot implement the trait expected
// gives way to the conversion of the receiver, here one over `O[A]` with an `OptionLike[O]`
// taken by the extension's own using clause (a frontend's `ifDefined` beside
// scalajs-react's `OptionExt`), whose implicit converter an implicit conversion supplies.
import scala.language.implicitConversions
class Builder:
  val out = collection.mutable.ListBuffer[String]()
trait TagMod:
  def applyTo(b: Builder): Unit
trait OptionLike[O[_]]:
  def fold[A, B](o: O[A], none: => B)(f: A => B): B
object OptionLike:
  given OptionLike[Option] with
    def fold[A, B](o: Option[A], none: => B)(f: A => B): B = o.fold(none)(f)
final class Text(s: String) extends TagMod:
  def applyTo(b: Builder): Unit = b.out += s
object Lib:
  implicit def vdomNodeFromString(s: String): TagMod = Text(s)
  implicit final class OptionExt[O[_], A](o: O[A])(implicit O: OptionLike[O]):
    def ifDefined(implicit ev: A => TagMod): TagMod = O.fold(o, Text("-"): TagMod)(ev)
object Prelude:
  extension [O[_], A](oa: O[A])(using O: OptionLike[O])
    def ifDefined(f: A => TagMod): TagMod = Lib.OptionExt(oa).ifDefined(using f)
import Lib.*
import Prelude.*
def render(name: Option[String]): TagMod = name.ifDefined
@main def run(): Unit =
  val b = Builder()
  render(Some("guest")).applyTo(b)
  render(None).applyTo(b)
  Option("x").ifDefined(s => Text(s + "!")).applyTo(b)
  println(b.out.mkString(","))
