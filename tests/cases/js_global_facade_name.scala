//> using platform js
// A Scala class named as a JS global that a native facade the program reaches binds (`@JSGlobal("Date")`): the
// output names the class apart (`Naming::avoid_taken`), so `new JSDate()` constructs the global `Date`. A facade the
// program does not reach takes no name (tests/modules/stdfacadename).
import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobal

@js.native
@JSGlobal("Date")
class JSDate() extends js.Object:
  def getFullYear(): Int = js.native

class Date(val y: Int)

@main def run(): Unit =
  val d = new Date(7)
  println(d.y)
  println(new JSDate().getFullYear() > 2000)
