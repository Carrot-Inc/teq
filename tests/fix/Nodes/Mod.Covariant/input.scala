/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Covariant, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Covariant

object Mod_Covariant:
  val marker = 0 // assert: DumpStructure.structure
  class C[+A]
  trait T[+B]
