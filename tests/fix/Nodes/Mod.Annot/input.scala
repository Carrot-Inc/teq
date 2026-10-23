/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Annot, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Annot

object Mod_Annot:
  val marker = 0 // assert: DumpStructure.structure
  @deprecated("x") def f = 1
  @deprecated("y", "1.0") @inline def g = 2
  class C(@deprecated x: Int)
