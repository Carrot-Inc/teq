// A local class that captures a local and has a second constructor: the capture is the enclosing instance alone.
class Host(val h: String):
  def make(): String =
    class Impl(tag: String):
      def this() = this("sub:")
      def show: String = tag + h
    new Impl().show
@main def run(): Unit = println(new Host("h").make())
