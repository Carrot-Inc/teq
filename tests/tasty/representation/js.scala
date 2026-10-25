// The oracle's Scala.js fixture (README.md): `js.Function1` is a trait with `js.Any`'s conversions both ways
// (Scala.js 1.22.0's Function.scala and Any.scala), the declared types observed.
package repr.js

import scala.scalajs.js

object Conversions:
  val ordinary: Int => Int = x => x + 1
  val foreign: js.Function1[Int, Int] = ordinary
  val back: Int => Int = foreign
  val literal: js.Function1[Int, Int] = (x: Int) => x + 2
