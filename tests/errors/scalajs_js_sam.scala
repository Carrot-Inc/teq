//> using platform js
// PrepJSInterop's transformStatOrExpr, the Closure case: an anonymous function typed against a JS trait that is no
// trait extending js.Function (scalac 3.8.4 --js; the message dotty's, at the start of the closure's span, 1-based).
// expect: 12:21: error: Using an anonymous function as a SAM for the JavaScript type NotFun is not allowed because it is not a trait extending js.Function. Use an anonymous class instead.
import scala.scalajs.js

trait NotFun extends js.Object:
  def run(x: Int): Int

object Main:
  def main(args: Array[String]): Unit =
    val a: NotFun = (x: Int) => x + 1
    println("done")
