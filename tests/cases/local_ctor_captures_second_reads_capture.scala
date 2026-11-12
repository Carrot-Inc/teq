// A local class that captures a local and has a second constructor: the second constructor's own argument reads the
// capture.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:" + p)
    def show: String = tag + p
  println(new Impl().show)
