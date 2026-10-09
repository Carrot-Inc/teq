// A JS trait's abstract var is a property: an anonymous JS object implements it with a var, and
// an assignment through the trait writes the property.
import scala.scalajs.js

trait Shape extends js.Object:
  var x: Int

object Main:
  def main(args: Array[String]): Unit =
    val s: Shape = new Shape { var x: Int = 1 }
    s.x = 2
    s.x += 1
    println(s.x)
    val t = new Shape { var x = 5 }
    println(js.JSON.stringify(t))
