/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Lit.Long, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Lit_Long

object Lit_Long:
  val marker = 0 // assert: DumpStructure.structure
  val a = 1L
  val b = 0x10L
  val c = -2L
  val d = 3l
