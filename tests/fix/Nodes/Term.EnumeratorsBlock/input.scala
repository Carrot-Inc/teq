/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.EnumeratorsBlock, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_EnumeratorsBlock

object Term_EnumeratorsBlock:
  val marker = 0 // assert: DumpStructure.structure
  val a = for x <- List(1) yield x
  val b = for { x <- List(1); y = x } yield y
  val c = for (x <- List(1)) yield x
