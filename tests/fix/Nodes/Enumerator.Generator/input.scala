/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Enumerator.Generator, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Enumerator_Generator

object Enumerator_Generator:
  val marker = 0 // assert: DumpStructure.structure
  val a = for x <- List(1); (y, z) <- List((1, 2)) yield x + y
