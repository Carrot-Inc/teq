// A JS setter of two parameters is an error, and the call of it is to be a method's, not a JavaScript
// assignment that has no place for a second argument. The expectation is scalac 3.8.4's message
// under Scala.js 1.22.0, at the definition's start where scalac reports it at the setter's name.
import scala.scalajs.js
class C extends js.Object:
  def x_=(a: Int, b: Int): Unit = ()
@main def run(): Unit =
  new C().x_=(1, 2)
