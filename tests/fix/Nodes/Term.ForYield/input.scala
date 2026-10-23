/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.ForYield, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_ForYield

object Term_ForYield:
  val marker = 0 // assert: DumpStructure.structure
  val a = for x <- List(1); if x > 0 yield x
  val b = for { x <- List(1) } yield x
