//> using platform js
// `js.Dynamic.literal(a = 1)` is a `js.Object with js.Dynamic` in Scala.js, so it is a js.Object, and its properties read dynamically
import scala.scalajs.js
def f(o: js.Object): String = "object"
@main def main(): Unit =
  println(f(js.Dynamic.literal(a = 1)))
  val o = js.Dynamic.literal(a = 1, b = "x")
  println(o.b)
  println(js.Object.keys(o).mkString(","))
