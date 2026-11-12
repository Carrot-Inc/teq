// A local class that captures a local and has a second constructor: the creation stands in a by-name argument.
def twice(x: => String): String = x + "," + x
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  println(twice(new Impl().show))
