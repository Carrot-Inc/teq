// A local class that captures a local and has a second constructor: the capture is a method's parameter.
def make(p: String): String =
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  new Impl().show
@main def run(): Unit = println(make("p"))
