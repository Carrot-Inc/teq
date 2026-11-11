//> using platform js
//> using jsVersion 1.22.0
// A native JS type's members called by their JSCallingConvention (JSCodeGen#genApplyJSMethodGeneric): a property
// set, bracket access read and write, a bracket call, a unary and a binary operator (R39.5-6; .expected from scalac
// 3.8.4 -scalajs).
import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native
trait Box extends js.Object:
  def x: Int = js.native
  def x_=(v: Int): Unit = js.native
  @JSBracketAccess
  def get(k: String): Int = js.native
  @JSBracketAccess
  def put(k: String, v: Int): Unit = js.native
  @JSBracketCall
  def callNamed(name: String)(a: Int): Int = js.native
  def unary_- : Int = js.native
  def +(that: Int): Int = js.native

object Main:
  def main(args: Array[String]): Unit =
    val b = js.Dynamic.literal(x = 1, y = 2, twice = ((a: Int) => a * 2): js.Function1[Int, Int], valueOf = (() => 10): js.Function0[Int]).asInstanceOf[Box]
    b.x = 5
    println(b.x)
    b.put("y", 7)
    println(b.get("y"))
    println(b.callNamed("twice")(21))
    println(-b)
    println(b + 1)
