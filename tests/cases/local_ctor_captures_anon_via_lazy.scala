// A local class that captures a local and has a second constructor: an anonymous subclass through the second
// constructor; the capture is a lazy val the creation does not run.
@main def run(): Unit =
  var calls = 0
  lazy val l: String = { calls += 1; "l" + calls }
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + l
  val a = new Impl() { override def show: String = "anon:" + super.show }
  println(calls)
  println(a.show)
  println(calls)
