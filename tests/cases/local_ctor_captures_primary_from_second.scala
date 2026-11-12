// A local class that captures a local and has a second constructor: a second constructor calls the primary; the
// capture is a local val.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  println(new Impl().show)
