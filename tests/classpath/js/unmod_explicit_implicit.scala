// jars: fixtures
// A library body passing an implicit clause explicitly after an implicit class's method,
// `(x ~=~ y)(ev)` (scalajs-react's `Reusability.tuple2`); the expectation is scalac 3.8.4's.
import fix.unmod.*

@main def main(): Unit =
  val r = UmReuse.pair[Int, String]
  println(r.test((1, "a"), (1, "a")))
  println(r.test((1, "a"), (2, "a")))
