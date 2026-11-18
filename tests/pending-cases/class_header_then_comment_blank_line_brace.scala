// A blank line inside a block comment between a class's header and a brace counts as a blank line: the brace
// is no body of the class but a block of the enclosing template, run once. scalac prints log:block.
object A:
  var log = ""
  class C(x: Int)
  /* c

  */
  { log += "block" }
  val c = (new C(1), new C(2))
@main def main(): Unit = { A.c; println("log:" + A.log) }
