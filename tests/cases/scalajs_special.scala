//> using platform js
// `js.special`: the JavaScript operators `delete`, `in`, `instanceof` and `===`.
import scala.scalajs.js

@main def main(): Unit =
  val o = js.Dynamic.literal(a = 1, b = 2).asInstanceOf[js.Dictionary[Int]]
  println(js.special.in("a", o))
  js.special.delete(o, "a")
  println(js.special.in("a", o))
  println(js.Object.keys(o.asInstanceOf[js.Object]).mkString(","))
  println(js.special.strictEquals(1, 1))
  println(js.special.instanceof(js.Array(1), js.Dynamic.global.Array))
