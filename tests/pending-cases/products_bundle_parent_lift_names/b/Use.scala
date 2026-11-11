// A downstream of a/ as tests/modules builds it, reduced from tests/modules/mixedparent. scalac 3.8.4 prints the
// .expected, and so does each of teq's runs. teq's Scala.js bundles of the whole program and of b/ over a/'s lean
// products differ in the names of D's parent arguments, lifted for their named order (tests/modules' bundle: one
// file, --split and --release alike). The whole bundle numbers them after B's lifted `z` (`z$2`, `x$2`), the other
// from one (`z$1`, `x$1`). The same on integration a097de53d9. mixedparent's bundle row shows it since 3a451f6219
// lets its JS products build complete: `note`'s `s + " "` was withheld before, and tests/modules.sh skips the bundle
// row when that build fails.
package bnb

import bna.*

@main def run(): Unit =
  val b = B(1)
  println()
  val d = D()
  println()
