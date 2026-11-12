// A local class that captures a local and has a second constructor: the class is defined inside a lambda.
@main def run(): Unit =
  val p = "p"
  val f = (n: Int) =>
    class Impl(tag: String):
      def this() = this("sub:")
      def show: String = tag + p + n
    new Impl().show
  println(List(1, 2).map(f))
