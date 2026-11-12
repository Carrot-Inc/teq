// A local class that captures a local and has a second constructor, made through the primary constructor.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  println(new Impl("x:").show)
