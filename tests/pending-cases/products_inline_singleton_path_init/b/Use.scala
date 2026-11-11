// A downstream of a/ as tests/modules builds it. a/'s inline `a14()` ascribes `O.v` to its singleton type `O.v.type`.
// scalac 3.8.4 prints the .expected: reading `O.v` initialises O first, and so do teq's whole runs. Over teq's products
// of a/ the JVM, Scala.js and interpreter runs print `true` alone: the expansion reads `v` without O's initialisation
// (tests/modules' jvm-run, js-run, interp-run; classes, subset and bundle differ too). The same before the SELECTtpt
// port (reviewer #14 on tip34). Found by reviewer #14 of the workers exports helper (push 9).
package sb

import sa.*

@main def run(): Unit =
  println(Paths.a14() eq O.I)
