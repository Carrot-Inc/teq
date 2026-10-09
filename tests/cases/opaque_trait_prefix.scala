// An opaque type declared in a trait, reached through objects deriving from the trait: the object
// is the prefix of the type (`Html.Tag`), so its members, the extensions and the conversion the
// trait declares, are in the type's implicit scope, each object's for its own copy of the type.
import scala.language.implicitConversions

final class Element(val tag: String, val children: List[String]):
  override def toString = children.mkString(s"<$tag>", "", s"</$tag>")

trait TagKit[Top]:
  final opaque type Tag[+N <: Top] = String
  def apply[N <: Top](name: String): Tag[N] = name
  extension [N <: Top](self: Tag[N])
    def name: String = self
    def apply(xs: String*): Element = Element(self, xs.toList)
  implicit def toElement[N <: Top](t: Tag[N]): Element = Element(t, Nil)

trait HtmlNode
trait SvgNode
object Html extends TagKit[HtmlNode]
object Svg extends TagKit[SvgNode]

object tags:
  def div = Html[HtmlNode]("div")
  def circle = Svg[SvgNode]("circle")

def render(e: Element): String = e.toString

@main def main(): Unit =
  println(tags.div("a", "b"))
  println(tags.div.name)
  println(render(tags.div))
  println(tags.circle("r"))
  println(render(tags.circle))
  val both: List[Element] = List(tags.div, tags.circle)
  println(both.map(_.tag))
