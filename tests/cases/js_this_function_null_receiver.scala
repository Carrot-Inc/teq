//> using platform js
//> using jsVersion 1.22.0
// A closure of js.ThisFunction1 receives its receiver as JavaScript's `this` and unboxes it to the first parameter's
// type (JSCodeGen#genClosure): a `Null` receiver is tested to be null, and `null` passes (.expected from scalac 3.8.4
// -scalajs).
import scala.scalajs.js

@main def main(): Unit =
  val f: js.ThisFunction1[Null, Int, Int] = (_: Null, x: Int) => 42 + x
  println(f(null, 1))
