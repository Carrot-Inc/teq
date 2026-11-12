// A local class that captures a local and has a second constructor: the creation stands inside a local def.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  def make(): Impl = new Impl()
  println(make().show)
