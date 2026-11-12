// A local class that captures a local and has a second constructor: a class nested in the local class, made outside
// it through its second constructor.
@main def run(): Unit =
  val p = "p"
  class Outer(tag: String):
    class Inner(k: Int):
      def this() = this(3)
      def show: String = tag + p + k
  val o = new Outer("out:")
  println(new o.Inner().show)
