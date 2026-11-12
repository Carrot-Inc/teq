// A downstream of a/ as tests/modules builds it,
// reduced from tests/modules/mixedparent. scalac 3.8.4
// prints the .expected. teq's Scala.js bundles of
// the whole program and of b/ over a/'s lean products
// are to agree in the names of D's parent arguments,
// lifted for their named order (tests/modules'
// bundle: one file, --split and --release alike).
package bnb

import bna.*

@main def run(): Unit =
  val b = B(1)
  println()
  val d = D()
  println()
