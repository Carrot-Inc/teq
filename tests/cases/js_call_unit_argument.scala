//> using platform js
//> using jsVersion 1.22.0
// A Unit argument of a call into JavaScript passed as undefined (JSCodeGen#genActualJSArgs' box by the parameter's
// type, R44.7-4; .expected from scalac 3.8.4 -scalajs).
import scala.scalajs.js
import scala.scalajs.js.annotation.*

object G:
  @js.native @JSGlobal("parseInt")
  def p(s: String): Unit = js.native
  @js.native @JSGlobal("String")
  def str(u: Unit): String = js.native

object Main:
  def main(args: Array[String]): Unit =
    println(G.str(G.p("7")))
