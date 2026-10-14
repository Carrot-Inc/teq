// An object inherits an abstract member from one trait and its implementation, an export's
// forwarder, from another: an import of the object, by wildcard or by name, binds the concrete
// member, as dotty joins a deferred and a concrete denotation to the concrete one
// (`Denotation.meet`, `isAsConcrete`).
object Html:
  def safeJson(s: String): String = s"<$s>"

trait AssetHelper:
  def safeJson(s: String): String
  def page(s: String): String = "page " + safeJson(s)

trait Helpers extends AssetHelper

trait AssetFullHelper:
  self: AssetHelper =>
  export Html.safeJson

object UiEnv extends Helpers with AssetFullHelper

object ByWildcard:
  import UiEnv.*
  def run(): String = safeJson("a") + " " + page("b")

object ByName:
  import UiEnv.safeJson
  def run(): String = safeJson("c")

@main def run(): Unit =
  println(ByWildcard.run())
  println(ByName.run())
  println(UiEnv.safeJson("d"))
