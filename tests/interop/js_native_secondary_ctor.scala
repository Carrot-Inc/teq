// A secondary constructor of a native JS class calls the primary constructor alone (PrepJSInterop's check of a
// constructor's body: the call's target, not its count of arguments): one that calls another secondary constructor is
// refused. scalac 3.8.4 -scalajs (`scala-cli compile --js --js-version 1.22.0 --scala 3.8.4`) reports the one error.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("Date")
class N(x: Int) extends js.Object:
  def this(s: String) = this(0)
  def this(b: Boolean) = this("x")
