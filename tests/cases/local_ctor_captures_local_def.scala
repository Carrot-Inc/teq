// A local class that captures a local and has a second constructor: the capture is a local def.
@main def run(): Unit =
  var calls = 0
  def p: String = { calls += 1; "p" + calls }
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  val a = new Impl()
  println(a.show)
  println(a.show)
