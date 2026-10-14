// An inherited class exported beside an object of the class's name: its constructor's outer instance
// is the object's module, a module reference (dotty's `ExplicitOuter`), not a call of a forwarder.
trait TagLite:
  class Tag[+N](val value: N)
object HtmlTagOf extends TagLite
export HtmlTagOf.{Tag as HtmlTagOf}
@main def run(): Unit = println(new HtmlTagOf(7).value)
