/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Lit.String, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Lit_String

object Lit_String:
  val marker = 0 // assert: DumpStructure.structure
  val a = "x"
  val b = "a\tb"
  val c = """multi
  line"""
  val d = ""
