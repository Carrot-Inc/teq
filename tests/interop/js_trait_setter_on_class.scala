// A JS trait's setter called on a class that implements it runs the class's setter: the call is the JavaScript
// assignment, and the class writes its setter as a JavaScript property setter that runs the body (Scala.js's
// JSExportsGen writes one). The expectation is scalac 3.8.4's under Scala.js 1.22.0
// (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
trait Sink extends js.Object:
  def x_=(v: Int): Unit
class C extends js.Object with Sink:
  var seen = 0
  def x_=(v: Int): Unit = seen = v
def set(s: Sink): Unit = s.x_=(7)
@main def run(): Unit =
  val c = new C()
  set(c)
  println(c.seen)
