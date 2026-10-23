/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Abstract, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Abstract

object Mod_Abstract:
  val marker = 0 // assert: DumpStructure.structure
  abstract class C:
    def f: Int
