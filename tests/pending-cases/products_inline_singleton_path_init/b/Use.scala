// A downstream of a/ as tests/modules builds it. a/'s inline `a14()`
// ascribes `O.v` to its singleton type `O.v.type`. scalac 3.8.4 prints the
// .expected: reading `O.v` initialises O first. Over teq's products of a/
// the JVM, Scala.js and interpreter runs are to print it too (tests/modules'
// jvm-run, js-run, interp-run; classes, subset and bundle are to agree too).
package sb

import sa.*

@main def run(): Unit =
  println(Paths.a14() eq O.I)
