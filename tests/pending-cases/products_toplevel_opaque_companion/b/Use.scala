// A downstream of a/ as tests/modules builds it. a/ has a top-level
// `opaque type OT` with a top-level `object OT` (its extension
// `raw`), used by an inline body. scalac 3.8.4 prints the .expected.
// Over a/'s lean products, b/'s Scala.js build and the interpreter
// are to print it too (tests/modules' js-run, interp-run).
package mq

import mp.*

@main def run(): Unit =
  println(Misc.m2(OT.mk(2)))
