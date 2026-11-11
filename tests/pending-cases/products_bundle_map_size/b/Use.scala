// A downstream of a/ as tests/modules builds it. a/ has an inline `m.size` on a Map and a plain `k + 1`. scalac 3.8.4
// prints the .expected, and so does each of teq's runs. teq's Scala.js bundles of the whole program and of b/ over a/'s
// products differ in content, not only names (tests/modules' bundle): the whole one carries std helpers (`$isInt`,
// `$isByte`, `$isShort`, `$newArray`, ...) the other lacks. Either method alone, or a List for the Map, gives identical
// bundles. The same before the SELECTtpt port (tip34). Found by reviewer #14 of the workers exports helper (push 9).
package rr3b

import rr3.*

@main def run(): Unit =
  println(R.r13(Map(1 -> 1)))
  println(R.p1(14))
