/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Sealed, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Sealed

object Mod_Sealed:
  val marker = 0 // assert: DumpStructure.structure
  sealed trait T
  sealed abstract class C
