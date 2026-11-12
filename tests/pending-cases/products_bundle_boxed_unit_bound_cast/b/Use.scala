// A downstream of a/ as
// tests/modules builds it.
// scalac 3.8.4 prints the
// .expected. a/'s `boxed`,
// a cast to a type parameter
// bounded by BoxedUnit, is
// to be written alike in
// teq's whole Scala.js bundle
// and in the bundle built
// over a/'s lean products
// (tests/modules' bundle: one
// file, --split and --release
// alike). scalac pickles
// the typer's `x.asInstanceOf[A]`.
// dotty's pickle precedes Erasure.
package bxb

import bxa.Lib

@main def run(): Unit =
  val u = Lib.boxed[scala.runtime.BoxedUnit](null)
  println("boxed")
