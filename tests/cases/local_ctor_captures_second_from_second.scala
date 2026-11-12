// A local class that captures a local and has a second constructor: a second constructor calls another second
// constructor.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String, n: Int):
    def this(tag: String) = this(tag, 1)
    def this() = this("sub:")
    def show: String = tag + p + n
  println(new Impl().show)
  println(new Impl("x:").show)
  println(new Impl("y:", 2).show)
