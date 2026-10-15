//> using platform js
// js_export_members with its unexported alternative taking a `List`: the classes an exported member's signature
// reaches are JavaScript's to call, `List`'s and through it `Ordering`'s inner `OrderingOps`, whose outer accessor
// the check of its class makes. No JavaScript names an outer accessor, so a watch session's rebuild, which finds it
// made, writes what a fresh build does (tests/split-watch.sh's case loop).
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
  def launch(id: String, flags: List[String]): String = "scala only"
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
    println(launch("z", Nil))
