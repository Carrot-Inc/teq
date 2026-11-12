// A downstream of a/ as tests/modules builds
// it, reduced from tests/modules/enumnamedlift.
// scalac 3.8.4 prints the .expected. teq's
// Scala.js bundles of the whole program
// and of b/ over a/'s lean products
// are to agree in b/'s calls of Key's
// constructor (tests/modules' bundle).
package bdb

import bda.*

object Use:
  def k = Key("a", 1)
  def l = Key(disabled = true, name = "b", keyType = 2)

@main def run(): Unit =
  println(Use.k)
  println(Use.l)
