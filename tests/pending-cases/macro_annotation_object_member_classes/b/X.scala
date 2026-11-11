// A downstream of a/ as tests/modules builds it, scalac 3.8.4 run with -experimental. A macro annotation on an
// object's member rewrites its body (`$rhs + 14`): scalac prints the .expected, and so does teq's interpreter over the
// whole program. teq's whole JVM build emits object O twice ("jvm backend: two classes are named miu11/O" and
// "miu11/O$"), its whole Scala.js build too (node: "Identifier 'O$M' has already been declared"). Over the products
// the annotation cannot run: the products build of a/ withholds `plus.transform`'s body ("elided: unstated a call
// of unapply with 0 arguments for 1 parameters"), and b/'s build reports "Failed to evaluate macro annotation
// 'new plus()'". Found by reviewer #13 of the workers exports helper (push 8); the same on integration b602956824.
package miu11

import scala.annotation.experimental
import mil.plus

@experimental
object O:
  @plus def v: Int = 1

@experimental
@main def run(): Unit =
  println(O.v)
