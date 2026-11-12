// A local class that captures a local and has a second constructor: a local case class.
@main def run(): Unit =
  val p = "p"
  case class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  println(new Impl().show)
  println(Impl("x:").show)
  println(new Impl() == Impl("sub:"))
