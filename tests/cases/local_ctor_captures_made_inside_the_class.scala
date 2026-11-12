// A local class that captures a local and has a second constructor: the creation stands inside the class itself.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
    def again: Impl = new Impl()
  println(new Impl("x:").again.show)
