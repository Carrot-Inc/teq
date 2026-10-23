/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Override, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Override

object Mod_Override:
  val marker = 0 // assert: DumpStructure.structure
  class B:
    def f = 1
  class C extends B:
    override def f = 2
