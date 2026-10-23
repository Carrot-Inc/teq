/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Lit.Int, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Lit_Int

object Lit_Int:
  val marker = 0 // assert: DumpStructure.structure
  val a = 1
  val b = 0x1F
  val c = 1_000
  val d = -1
  val e = 0
