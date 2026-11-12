// A local class that captures a local and has a second constructor: a local class of a class's method: it captures a
// parameter and the enclosing instance.
class Host(val h: String):
  def make(p: String): String =
    class Impl(tag: String):
      def this() = this("sub:")
      def show: String = tag + p + h
    new Impl().show
@main def run(): Unit = println(new Host("h").make("p"))
