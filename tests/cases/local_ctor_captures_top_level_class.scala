// A top-level class with a second constructor.
class Impl(tag: String):
  def this() = this("sub:")
  def show: String = tag
@main def run(): Unit =
  println(new Impl().show)
