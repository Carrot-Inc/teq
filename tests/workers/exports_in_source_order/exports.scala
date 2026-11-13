// A module's exports stand in the source order of their definitions at every worker count: an object
// written after its enum or its case class where it is written, at the top level and inside an object,
// and an exported object before the exports inside it.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSExportTopLevel

enum Shade { case Dark }
@JSExportTopLevel("between")
def between(): Int = 1
@JSExportTopLevel("Shade")
object Shade:
  @JSExportTopLevel("deep")
  def deep(): Int = 2
@JSExportTopLevel("zeta")
def zeta(): Int = 3
@JSExportTopLevel("Alpha")
object Alpha { val n: Int = 4 }
@JSExportTopLevel("aval")
val aval: Int = 5
object Bin:
  enum Grain { case Fine }
  case class Slot(n: Int)
  @JSExportTopLevel("sift")
  def sift(): Int = 8
  @JSExportTopLevel("Grain")
  object Grain:
    @JSExportTopLevel("coarse")
    def coarse(): Int = 9
  @JSExportTopLevel("fit")
  def fit(): Int = 10
  @JSExportTopLevel("Slot")
  object Slot { val width: Int = 11 }
  @JSExportTopLevel("last")
  def last(): Int = 12
@main def run(): Unit =
  println(between() + Shade.deep() + zeta() + Alpha.n + aval + Bin.sift() + Bin.Grain.coarse() + Bin.fit() + Bin.Slot.width + Bin.last())
