// An exported val, then an exported object written after its enum, inside one object: the module makes
// its exported values in the order they are written, the val's object before the later object, as
// Scala.js does for this program.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSExportTopLevel

object Bin:
  enum Grain { case Fine }
  @JSExportTopLevel("first")
  val first: Int = { println("Bin is made"); 1 }
  @JSExportTopLevel("Grain")
  object Grain:
    println("Grain is made")
    val n: Int = 2
