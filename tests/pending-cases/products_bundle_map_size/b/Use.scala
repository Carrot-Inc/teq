// A downstream of a/ as tests/modules builds it. a/
// has an inline `m.size` on a Map and a plain `k + 1`.
// scalac 3.8.4 prints the .expected. teq's Scala.js
// bundles of the whole program and of b/ over a/'s
// products are to be the same (tests/modules' bundle).
package rr3b

import rr3.*

@main def run(): Unit =
  println(R.r13(Map(1 -> 1)))
  println(R.p1(14))
