/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Using, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Using

object Mod_Using:
  val marker = 0 // assert: DumpStructure.structure
  def f(using a: Int) = a
  given Int = 1
  val b = f(using 2)
