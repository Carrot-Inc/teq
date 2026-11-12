// A local class with a second constructor that captures nothing.
@main def run(): Unit =
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag
  println(new Impl().show)
