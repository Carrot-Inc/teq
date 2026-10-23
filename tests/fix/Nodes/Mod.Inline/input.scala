/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Inline, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Inline

object Mod_Inline:
  val marker = 0 // assert: DumpStructure.structure
  inline val a = 1
  inline def f(inline x: Int) = x
