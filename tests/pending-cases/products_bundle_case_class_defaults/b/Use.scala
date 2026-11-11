// A downstream of a/ as tests/modules builds it, reduced from tests/modules/enumnamedlift. scalac 3.8.4 prints the
// .expected, and so does each of teq's runs. teq's Scala.js bundles of the whole program and of b/ over a/'s lean
// products differ in b/'s calls of Key's constructor (tests/modules' bundle). The whole bundle passes the omitted
// arguments as the default getters' calls (`Key.$default$3()`, `Key.$default$4()`) and so carries the companion's
// module class; the other passes none, or `undefined` before a given one, and leaves them to the JS constructor's
// defaults. teq's JVM classes of b/ call the getters in both builds. The same on integration a097de53d9, where
// enumnamedlift's JS products build failed on a withheld body and tests/modules.sh skipped its bundle row.
package bdb

import bda.*

object Use:
  def k = Key("a", 1)
  def l = Key(disabled = true, name = "b", keyType = 2)

@main def run(): Unit =
  println(Use.k)
  println(Use.l)
