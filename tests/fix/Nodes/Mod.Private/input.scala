/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Private: unqualified, qualified and this-qualified, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Private

object Mod_Private:
  val marker = 0 // assert: DumpStructure.structure
  private val a = 1
  private[Mod_Private] val b = 2
  class C:
    private[this] val c = 3
  class D private[/* note */ Mod_Private] (x: Int)
