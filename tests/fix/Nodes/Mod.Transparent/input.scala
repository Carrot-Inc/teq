/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Transparent, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Transparent

object Mod_Transparent:
  val marker = 0 // assert: DumpStructure.structure
  transparent inline def f = 1
  transparent trait T
