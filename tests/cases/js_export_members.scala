//> using platform js
// Members exported to JavaScript with @JSExport, as tyrian's `TyrianApp.launch` is from its sandbox
// page: a trait's alternatives of one name are one JS method, which takes the alternative the
// number of arguments and their run-time types select (an element, a string, a number), a
// parameterless def is a property, an override answers, and an alternative without the
// annotation is not exported. An object exported with @JSExportTopLevel is its instance.
import scala.scalajs.js
import scala.scalajs.js.annotation.{JSExport, JSExportTopLevel}

trait App:
  def name: String
  @JSExport
  def launch(id: String): String = name + " in #" + id
  @JSExport
  def launch(id: String, flags: js.Dictionary[String]): String = name + " in #" + id + " with " + flags("message")
  @JSExport
  def launch(n: Int): String = name + " " + n + " times"
  @JSExport
  def launch(d: js.Date): String = name + " at " + d.getUTCFullYear()
  def launch(flag: Boolean): String = "scala only"
  @JSExport("title")
  def heading: String = "the " + name

class Base:
  @JSExport
  def greet(who: String): String = "hello " + who

class Polite extends Base:
  override def greet(who: String): String = "good day " + who

@JSExportTopLevel("TheApp")
object Main extends App:
  def name = "main"

  def main(args: Array[String]): Unit =
    val app = Main.asInstanceOf[js.Dynamic]
    println(app.launch("x"))
    println(app.launch(3))
    println(app.launch("y", js.Dictionary("message" -> "hi")))
    println(app.launch(new js.Date(0)))
    println(app.title)
    try app.launch(true)
    catch case e: js.JavaScriptException => println("no alternative: " + e.exception.asInstanceOf[js.Dynamic].message)
    println(new Polite().asInstanceOf[js.Dynamic].greet("you"))
    println(launch(false))
