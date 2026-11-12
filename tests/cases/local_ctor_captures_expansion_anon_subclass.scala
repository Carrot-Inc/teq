// A local class that captures a local and has a second constructor: an anonymous subclass through the second
// constructor in an inline method's expansion.
inline def mk(inline pre: String): String =
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + pre
  val a = new Impl() { override def show: String = "anon:" + super.show }
  a.show
@main def run(): Unit =
  val p = "p"
  println(mk(p))
