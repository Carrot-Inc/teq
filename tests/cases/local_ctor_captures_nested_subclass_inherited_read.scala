// A local class that captures a local and has a second constructor: a subclass of a class nested in the local class,
// through that one's second constructor, reads the capture in an inherited method.
@main def run(): Unit =
  val p = "p"
  class Outer(tag: String):
    class Inner(k: Int):
      def this() = this(3)
      def show: String = tag + p + k
    class Sub extends Inner()
    def sub: Inner = new Sub
  println(new Outer("out:").sub.show)
