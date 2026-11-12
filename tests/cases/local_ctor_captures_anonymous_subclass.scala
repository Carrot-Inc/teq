// A local class that captures a local and has a second constructor: an anonymous subclass made through the second
// constructor.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  val a = new Impl() { override def show: String = "anon:" + super.show }
  println(a.show)
