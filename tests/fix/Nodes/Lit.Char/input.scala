/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Lit.Char, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Lit_Char

object Lit_Char:
  val marker = 0 // assert: DumpStructure.structure
  val a = 'a'
  val b = '\n'
  val c = '\u0041'
