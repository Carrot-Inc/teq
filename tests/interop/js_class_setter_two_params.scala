// A JS setter of two parameters is the preparation's error, reported once the typing is joined; the call typed before
// it is a method's, not a JavaScript assignment that has no place for a second argument. scalac 3.8.4 under Scala.js
// 1.22.0 reports the same message, at the setter's name where teq reports its definition's start.
import scala.scalajs.js
class C extends js.Object:
  def x_=(a: Int, b: Int): Unit = ()
@main def run(): Unit =
  new C().x_=(1, 2)
