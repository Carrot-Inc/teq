// A local class that captures a local and has a second constructor: the class's initialiser writes the captured var.
@main def run(): Unit =
  var p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    p = p + "!"
    def show: String = tag + p
  val a = new Impl()
  println(p)
  p = p + "?"
  println(a.show)
