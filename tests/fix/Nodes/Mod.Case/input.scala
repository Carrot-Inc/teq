/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Case, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Case

object Mod_Case:
  val marker = 0 // assert: DumpStructure.structure
  case class C(x: Int)
  case object O
