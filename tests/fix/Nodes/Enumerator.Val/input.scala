/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Enumerator.Val, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Enumerator_Val

object Enumerator_Val:
  val marker = 0 // assert: DumpStructure.structure
  val a = for x <- List(1); y = x + 1 yield y
