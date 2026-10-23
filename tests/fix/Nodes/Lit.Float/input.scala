/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Lit.Float, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Lit_Float

object Lit_Float:
  val marker = 0 // assert: DumpStructure.structure
  val a = 1.5f
  val b = 2F
  val c = -0.5f
  val d = 1e2f
  val e = -1e-7f
  val f = 0.0f
  val g = 1.50f
