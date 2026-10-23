/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Tuple, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Tuple

object Term_Tuple:
  val marker = 0 // assert: DumpStructure.structure
  val a = (1, 2)
  val b = (1, "a", true)
