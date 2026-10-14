// A class made through an alias whose expansion carries its own prefix (`new API.Tag` of an export's
// `type Tag = HtmlTagOf.Tag`): the enclosing instance is the class type's prefix, the object's module,
// as dotty's `ExplicitOuter.outerPrefix` dealiases the type, not the written `API`.
trait TagLite:
  def name: String
  class Tag[+N](val value: N):
    def show = s"$name:$value"
object HtmlTagOf extends TagLite:
  def name = "html"
object API:
  def name = "api"
  export HtmlTagOf.Tag
export HtmlTagOf.{Tag as Top}
@main def run(): Unit =
  val x: HtmlTagOf.Tag[String] = new API.Tag("ok")
  println(x.show)
  import API.*
  println(new Tag(1).show)
  println(new Top(2).show)
