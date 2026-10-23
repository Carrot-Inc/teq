/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Lazy, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Lazy

object Mod_Lazy:
  val marker = 0 // assert: DumpStructure.structure
  lazy val a = 1
  private lazy val b = 2
