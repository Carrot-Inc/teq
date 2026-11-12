// A local class that captures a local and has a second constructor: the creation stands inside a lambda.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String):
    def this() = this("sub:")
    def show: String = tag + p
  val f = (n: Int) => new Impl().show + n
  println(List(1, 2).map(f))
