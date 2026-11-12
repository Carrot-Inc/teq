// A local class that captures a local and has a second constructor: the class is defined inside a local def.
@main def run(): Unit =
  val p = "p"
  def make(n: Int): String =
    class Impl(tag: String):
      def this() = this("sub:")
      def show: String = tag + p + n
    new Impl().show
  println(make(1) + make(2))
