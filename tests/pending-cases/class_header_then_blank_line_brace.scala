// A brace on the line after a class's header and a blank line is no body of the class: it is a block, a
// statement of the enclosing template, run once when that is initialized. scalac prints log:block.
object A:
  var log = ""
  class C(x: Int)

  { log += "block" }
  val c = (new C(1), new C(2))
@main def main(): Unit = { A.c; println("log:" + A.log) }
