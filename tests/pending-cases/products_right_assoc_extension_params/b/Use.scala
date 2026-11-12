// A downstream of a/ as tests/modules builds
// it, reduced from tests/modules/defaultsplan.
// scalac 3.8.4 prints the .expected.
// Over a/'s products teq is to bind
// a right-associative extension's two
// parameters as scalac does: the Scala.js
// and interpreter runs (js-run, interp-run)
// and the JVM split run (jvm-run) are to
// print `R-A` and `R~A`, as scalac prints
// them. scalac's pickle writes `--:`'s
// parameters in dotty's encoded order,
// (a)(r). The bundles are to be the same
// too (bundle), and the whole and split
// builds' classes of b/ (classes, subset).
package rdb

import rda.Api
import rda.Api.*

@main def run(): Unit =
  println(("A" ++: "R")())
  println(Api.local())
  println("A" --: "R")
  println("A" ~~: "R")
