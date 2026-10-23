/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Enumerator.Guard, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Enumerator_Guard

object Enumerator_Guard:
  val marker = 0 // assert: DumpStructure.structure
  val a = for x <- List(1) if x > 0; if x < 2 yield x
