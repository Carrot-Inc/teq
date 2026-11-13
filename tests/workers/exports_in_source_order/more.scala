// A second file's exports follow the first file's.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSExportTopLevel

enum Tone { case Low }
@JSExportTopLevel("second")
def second(): Int = 6
@JSExportTopLevel("Tone")
object Tone { val n: Int = 7 }
