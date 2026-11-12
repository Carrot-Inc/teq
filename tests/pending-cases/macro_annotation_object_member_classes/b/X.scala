// A downstream of a/ as tests/modules builds it,
// scalac 3.8.4 run with -experimental. A macro
// annotation on an object's member rewrites its body
// (`$rhs + 14`): scalac prints the .expected, and teq's
// whole JVM build, its whole Scala.js build and b/'s
// build over a/'s products are to print it too. Found by
// reviewer #13 of the workers exports helper (push 8).
package miu11

import scala.annotation.experimental
import mil.plus

@experimental
object O:
  @plus def v: Int = 1

@experimental
@main def run(): Unit =
  println(O.v)
