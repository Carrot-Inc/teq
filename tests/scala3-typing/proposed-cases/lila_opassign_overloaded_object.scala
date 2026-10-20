// `f(args) := v` where `f` names both an object and a method (scalatags' `object data` +
// `def data(s)`, `object aria` + lila's `def aria(key)`): scalac types `f(args).:=(v)` first.
class Attr(val name: String):
  def :=(v: String): String = s"$name=$v"

trait Attrs:                       // scalatags.generic.Attrs / GlobalAttrs
  object data
  def data(suffix: String): Attr = Attr("data-" + suffix)
  object aria

trait Prefix:                      // lila's ScalatagsPrefix: `object st extends ... Attrs`
  object st extends Attrs

trait LilaAttrs:                   // lila's ScalatagsAttrs
  def aria(key: String): Attr = Attr("aria-" + key)

trait Template extends Attrs with LilaAttrs with Prefix
object Template extends Template

@main def run() =
  println(Template.st.data("action") := "x")
  println(Template.aria("pressed") := "false")
