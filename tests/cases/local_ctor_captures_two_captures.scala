// A local class that captures a local and has a second constructor: two captures.
@main def run(): Unit =
  val p = "p"
  val q = 7
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p + q
  println(new Impl().show)
