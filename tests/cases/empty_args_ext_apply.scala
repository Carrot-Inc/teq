// `f()` for a parameterless `f` applies the result's `apply`, an extension method as well: one of
// the implicit scope of the result's type, and one in lexical scope (scalac: "div()", "br()",
// "p(x)", "7").
object HtmlTagOf:
  opaque type Tag = String
  def apply(name: String): Tag = name
  extension (self: Tag) def apply(xs: String*): String = s"$self(${xs.mkString(",")})"

trait Kit:
  final opaque type Tag = String
  def make(name: String): Tag = name
  extension (self: Tag) def apply(xs: String*): String = s"$self(${xs.mkString(",")})"
object Html extends Kit

final class Counter(val n: Int)
extension (c: Counter) def apply(): Int = c.n

object tags:
  def div = HtmlTagOf("div")
  def br = Html.make("br")
  def p = Html.make("p")
  def seven = Counter(7)

@main def main(): Unit =
  println(tags.div())
  println(tags.br())
  println(tags.p("x"))
  println(tags.seven())
