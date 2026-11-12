// A local class that captures a local and has a second constructor: the class stands in an inline method's expansion
// and captures the call site's local.
inline def mk(inline pre: String): String =
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + pre
  new Impl().show
@main def run(): Unit =
  val p = "p"
  println(mk(p))
