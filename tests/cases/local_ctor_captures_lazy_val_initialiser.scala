// A local class that captures a local and has a second constructor: the creation stands in a lazy val's initialiser.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  lazy val a: Impl = new Impl()
  println(a.show)
