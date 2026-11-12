// A local class that captures a local and has a second constructor: a second constructor with a default argument.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String, n: Int):
    def this(tag: String = "sub:") = this(tag, 1)
    def show: String = tag + p + n
  println(new Impl().show)
  println(new Impl("x:").show)
