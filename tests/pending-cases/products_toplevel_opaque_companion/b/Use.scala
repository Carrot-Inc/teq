// A downstream of a/ as tests/modules builds it. a/ has a top-level `opaque type OT` with a top-level `object OT` (its
// extension `raw`), used by an inline body. scalac 3.8.4 prints the .expected, and so do teq's JVM runs and its whole
// runs. Over a/'s lean products, b/'s Scala.js build and the interpreter panic: "a package-object union has associated
// files" (src/typer/class_denotations.rs) (tests/modules' js-run, interp-run). The same before the SELECTtpt port
// (reviewer #14 on tip34). Found by reviewer #14 of the workers exports helper (push 9).
package mq

import mp.*

@main def run(): Unit =
  println(Misc.m2(OT.mk(2)))
