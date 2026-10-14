// An object's inherited class exported at the top level: its outer instance is the object's module.
trait TagLite:
  class Tag[+N](val value: N)
object HtmlTagOf extends TagLite
export HtmlTagOf.Tag
@main def run(): Unit = println(new Tag(7).value)
