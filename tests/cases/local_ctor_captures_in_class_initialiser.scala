// A local class that captures a local and has a second constructor: the class stands in a class's initialiser.
class Host:
  val x: String =
    val p = "p"
    class Impl(tag: String):
      def this() = this("sub:")
      def show: String = tag + p
    new Impl().show
@main def run(): Unit = println(new Host().x)
