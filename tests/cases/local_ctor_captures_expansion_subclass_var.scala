// A local class that captures a local and has a second constructor: a subclass through the parent's second
// constructor in an inline method's expansion; the capture is the call site's var.
inline def mk(inline bump: Unit, inline read: String): String =
  class Base(tag: String):
    def this() = this("base:")
    def show: String = tag + read
  class Impl extends Base()
  val a = new Impl
  bump
  a.show
@main def run(): Unit =
  var p = "p"
  println(mk({ p = p + "!" }, p))
