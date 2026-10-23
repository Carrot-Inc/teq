/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Infix, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Infix

object Mod_Infix:
  val marker = 0 // assert: DumpStructure.structure
  infix def op(a: Int, b: Int) = a + b
  infix type or[A, B] = Either[A, B]
