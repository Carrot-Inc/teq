// A local class that captures a local and has a second constructor: a class nested in the local class extends another
// through that one's second constructor.
@main def run(): Unit =
  val p = "p"
  class Outer(tag: String):
    class Inner(k: Int):
      def this() = this(3)
      def show: String = tag + p + k
    class Sub extends Inner():
      override def show: String = "sub:" + super.show
    def sub: Inner = new Sub
  println(new Outer("out:").sub.show)
