// Where overloads would need a choice at run time in JavaScript: a class whose instances are
// plain JS objects has one method per name, and so has a module for each exported name.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSExportTopLevel

class Handler extends js.Object:
  def handle(code: Int): String = "code"
  def handle(text: String): String = "text"

object Api:
  @JSExportTopLevel("render")
  def render(n: Int): String = "#" + n
  @JSExportTopLevel("render")
  def render(s: String): String = s
  @JSExportTopLevel("renderBoth")
  def render(n: Int, s: String): String = s * n

@main def run(): Unit = println(Api.render(1))
