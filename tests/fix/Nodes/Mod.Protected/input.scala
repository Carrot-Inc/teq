/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Protected, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Protected

object Mod_Protected:
  val marker = 0 // assert: DumpStructure.structure
  class C:
    protected val a = 1
    protected[Mod_Protected] val b = 2
    protected[this] val c = 3
