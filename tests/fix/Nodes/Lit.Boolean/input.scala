/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Lit.Boolean, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Lit_Boolean

object Lit_Boolean:
  val marker = 0 // assert: DumpStructure.structure
  val a = true
  val b = false
  val c = !true
