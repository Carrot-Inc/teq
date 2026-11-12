// A local class that captures a local and has a second constructor: a second constructor's default argument reads the
// capture.
@main def run(): Unit =
  val p = "p"
  class Impl(tag: String, n: Int):
    def this(tag: String = p + ":") = this(tag, 1)
    def show: String = tag + p + n
  println(new Impl().show)
  println(new Impl("x:").show)
