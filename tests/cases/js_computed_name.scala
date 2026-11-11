//> using platform js
//> using jsVersion 1.22.0
// A member named by a computed @JSName (JSSymUtils.jsName's Computed, JSCodeGen#genComputedJSName): keyed by the
// symbol, called as a[Syms.k](), no property "bar" (R44.6-10; .expected from scalac 3.8.4 -scalajs).
import scala.scalajs.js
import scala.scalajs.js.annotation.*

object Syms:
  val k = js.Symbol()

class A extends js.Object:
  @JSName(Syms.k)
  def bar(): Int = 1

object Main:
  def main(args: Array[String]): Unit =
    val a = new A
    println(a.bar())
    println(js.isUndefined(a.asInstanceOf[js.Dynamic].selectDynamic("bar")))
