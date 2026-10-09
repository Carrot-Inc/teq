//> using platform js
// `js.Dynamic.literal` with named arguments and with pairs: Scala.js's `literal` is an object
// applied dynamically (`applyDynamicNamed`), and the named form is an object literal.
import scala.scalajs.js

object Main:
  def main(args: Array[String]): Unit =
    val frame = js.Dynamic.literal(transform = "scale(1)", color = "#888", offset = 0.25)
    println("" + frame.transform + " " + frame.color + " " + frame.offset)
    val pairs = js.Dynamic.literal("a" -> 1, "b" -> "x")
    println("" + pairs.a + " " + pairs.b)
    println(js.Object.keys(js.Dynamic.literal().asInstanceOf[js.Object]).length)
    val frames = js.Array(js.Dynamic.literal(opacity = 0), js.Dynamic.literal(opacity = 1))
    println("" + frames.length + " " + frames(1).opacity)
