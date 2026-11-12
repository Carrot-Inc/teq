// A local class that captures a local and has a second constructor: an anonymous subclass through the second
// constructor that captures another local first; the capture is a var.
@main def run(): Unit =
  val z = "z"
  var p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  val a = new Impl() { override def show: String = z + ":" + super.show }
  p = "q"
  println(a.show)
