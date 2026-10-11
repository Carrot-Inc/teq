//> using platform js
// Casts to a JavaScript type the std holds as an alias of a Scala type, named through an opaque type
// (inside its scope and outside it), a type member of a value's class and a chain of local aliases:
// Scala.js leaves them untested, JSCodeGen.genAsInstanceOf reading the type after erasure.
import scala.scalajs.js

object Hiding:
  opaque type Numbers = js.Array[Int]
  def numbers(x: Any): Any = x.asInstanceOf[Numbers]

class Shape:
  type Values = js.Array[Int]

@main def run(): Unit =
  val o = js.Dynamic.literal(length = 2)
  val shape = new Shape
  type L = js.Array[Int]
  type M = L
  def length(x: Any): Int = x.asInstanceOf[js.Array[Int]].length
  println(length(Hiding.numbers(o)))
  println(length(o.asInstanceOf[Hiding.Numbers]))
  println(length(o.asInstanceOf[shape.Values]))
  println(length(o.asInstanceOf[M]))
