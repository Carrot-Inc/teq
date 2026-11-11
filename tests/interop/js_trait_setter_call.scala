// A setter of a JS trait called by its name (`b.x_=(7)`) on an object that holds the property alone (a literal) is the
// JavaScript assignment `b.x = 7`, as on a class, whose setter is a JavaScript property setter. The expectation is
// scalac 3.8.4's under Scala.js 1.22.0 (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`), not written by hand.
import scala.scalajs.js

trait Box extends js.Object:
  def x: Int
  def x_=(v: Int): Unit

object Main:
  def main(args: Array[String]): Unit =
    val b = js.Dynamic.literal(x = 1).asInstanceOf[Box]
    b.x_=(7)
    println(b.x)
