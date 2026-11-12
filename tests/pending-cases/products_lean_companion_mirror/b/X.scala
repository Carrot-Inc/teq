// A downstream of a/ as tests/modules builds it. An inline
// method of a/ makes a value through the mirror it summons
// (`m.fromProduct`), for a case class with a written companion
// (`C`) and one without (`D`). scalac 3.8.4 prints the
// .expected. Over teq's products of a/ built with the lean
// std, the Scala.js and interpreter runs are to print it
// too (tests/modules' js-run, interp-run). The whole build's
// classes are to be the split build's (classes, subset, bundle).
package zmb
import zma.*
@main def run(): Unit =
  println(Defs.make[C])
  println(Defs.make[D])
