// A local class that captures a local and has a second constructor: a class nested in the local class, both made
// through their second constructors.
@main def run(): Unit =
  val p = "p"
  class Outer(tag: String):
    def this() = this("out:")
    class Inner(k: Int):
      def this() = this(3)
      def show: String = tag + p + k
    def inner: Inner = new Inner()
  println(new Outer().inner.show)
