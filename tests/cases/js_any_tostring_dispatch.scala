//> using platform js
//> using jsVersion 1.22.0

import scala.scalajs.js

// The program's `toString` of a value that may be a JavaScript value is Scala.js's dispatch (`$dp_toString__T`): an
// object's own, a Scala instance's as its class gives it, whether written on an `Any`, in an inline body over `T :=
// Any` or after a transparent expansion keeps it universal.
// The unit, arrays and functions (docs/COMPATIBILITY.md) are not here.
case class P(n: Int)

transparent inline def tr[T](y: T): String = y.toString
inline def il[T](y: T): String = y.toString
def src(y: Any): String = y.toString

@main def run(): Unit =
  val o: Any = js.Dynamic.literal(a = 1)
  val p: Any = P(3)
  println("obj " + src(o) + " | " + tr[Any](o) + " | " + il[Any](o) + " | " + tr[Matchable](o) + " | " + o.toString)
  println("scala " + src(p) + " | " + tr[Any](p) + " | " + il[Any](p) + " | " + tr[P](P(4)) + " | " + tr[Int](5) + " | " + tr[String]("s"))
