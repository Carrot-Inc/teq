// A local class that captures a local and has a second constructor: the capture is a local var, written after the
// creation and from inside the class.
@main def run(): Unit =
  var p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
    def set(s: String): Unit = p = s
  val a = new Impl()
  p = "q"
  println(a.show)
  a.set("r")
  println(p)
