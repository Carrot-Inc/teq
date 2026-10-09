// A cast to `scala.runtime.BoxedUnit` is a cast to `Unit` (TypeTestsCasts' `BoxedUnitClass`
// branch, after the ascription): the operand, then `()`; an operand typed `Null` conforms and
// stays `null`, as an ascription leaves it.
var n = 0
def value: Any = { n += 1; "text" }
def attempt(label: String)(f: => scala.runtime.BoxedUnit): Unit =
  try { val x = f; println(label + " null=" + (x == null)) }
  catch case _: ClassCastException => println(label + " CCE")
@main def run(): Unit =
  attempt("unit") { ((): Any).asInstanceOf[scala.runtime.BoxedUnit] }
  attempt("any text") { value.asInstanceOf[scala.runtime.BoxedUnit] }; println(n)
  attempt("primitive") { 7.asInstanceOf[scala.runtime.BoxedUnit] }
  attempt("any null") { (null: Any).asInstanceOf[scala.runtime.BoxedUnit] }
  attempt("static null") { null.asInstanceOf[scala.runtime.BoxedUnit] }
  val u: Any = (null: Any).asInstanceOf[scala.runtime.BoxedUnit]
  println(u == ())
