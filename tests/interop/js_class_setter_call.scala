// A program JS class's own setter called by its name runs its body: the call is the JavaScript assignment, and the class
// writes the setter as a JavaScript property setter, which runs the body (an assignment of a plain property would skip
// it and leave `seen` at 0). The expectation is scalac 3.8.4's under Scala.js 1.22.0
// (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
class C extends js.Object:
  var seen = 0
  def x_=(v: Int): Unit = seen = v
@main def run(): Unit =
  val c = new C()
  c.x_=(7)
  println(c.seen)
