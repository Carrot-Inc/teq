//> using platform js
// The Scala.js facade layer of the std: `scala.scalajs.js` is entered when a program imports it,
// and js.Array is the JavaScript array.
import scala.scalajs.js

@main def run(): Unit =
  val a = js.Array(1, 2)
  a.push(3)
  println(a.length)
  println(a.map(_ * 2).toList)
  println(js.Array.isArray(a))
  val d = js.Dictionary("x" -> 1, "y" -> 2)
  println(d("y"))
  println(js.typeOf("s"))
  println(js.isUndefined(js.undefined))
