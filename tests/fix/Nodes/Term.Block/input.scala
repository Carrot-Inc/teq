/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Block, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Block

object Term_Block:
  val marker = 0 // assert: DumpStructure.structure
  val a = { val b = 1; b }
  val c = {
    1
  }
