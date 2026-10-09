//> using platform js
// Methods of the `scala.scalajs.js` facades whose trailing parameters default to `js.native`:
// omitted, they are left out of the JavaScript call.
import scala.scalajs.js

@main def main(): Unit =
  println(js.JSON.stringify(js.Dynamic.literal(a = 1)))
  println(js.JSON.stringify(js.Dynamic.literal(a = 1, b = "x"), null, 1))
  println(js.Date.UTC(2020, 1))
  println(js.Date.UTC(2020, 1, 2, 3))
