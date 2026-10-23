/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.ValParam, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_ValParam

object Mod_ValParam:
  val marker = 0 // assert: DumpStructure.structure
  class C(val a: Int)
  case class D(val b: Int)
