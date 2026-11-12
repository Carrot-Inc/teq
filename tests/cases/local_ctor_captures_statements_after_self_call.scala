// A local class that captures a local and has a second constructor: statements after a second constructor's self call
// read and write a captured var.
@main def run(): Unit =
  var p = "p"
  class Impl(tag: String):
    var seen = ""
    def this() = { this("sub:"); seen = p; p = p + "!" }
    def show: String = tag + p + seen
  println(new Impl().show)
  println(p)
