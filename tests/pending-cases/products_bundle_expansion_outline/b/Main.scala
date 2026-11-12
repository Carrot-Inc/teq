// Downstream of a/ as tests/modules builds it.
// scalac 3.8.4 prints the .expected, on the JVM
// and on Scala.js. a/'s `Use.kinds` holds two
// expansions of one shape of the transparent
// `kind`. teq's Scala.js bundle of the whole
// program and the bundle built over a/'s lean
// products are to write them alike (tests/modules'
// bundle: one file, --split and --release alike).
package rnb

@main def run(): Unit =
  println(rna.Use.kinds(1))
  println(rna.Use.kinds(5))
