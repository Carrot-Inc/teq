/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Enumerator.CaseGenerator, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Enumerator_CaseGenerator

object Enumerator_CaseGenerator:
  val marker = 0 // assert: DumpStructure.structure
  val a = for case (x, y) <- List((1, 2)) yield x
