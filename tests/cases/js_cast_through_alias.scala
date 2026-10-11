//> using platform js
// A cast to a JavaScript type the std holds as an alias of a Scala type, written through aliases of
// it: Scala.js leaves it untested (JSCodeGen.genAsInstanceOf, by the type's symbol after dealiasing).
import scala.scalajs.js

type Values = js.Array[Int]
type Again = Values

object Shapes:
  type Numbers = js.Array[Int]

@main def run(): Unit =
  val o = js.Dynamic.literal(length = 2)
  val a = o.asInstanceOf[js.Array[Int]]
  val b = o.asInstanceOf[Values]
  val c = o.asInstanceOf[Again]
  val d = o.asInstanceOf[Shapes.Numbers]
  println(a.length + b.length + c.length + d.length)
