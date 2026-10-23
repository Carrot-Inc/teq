/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Importee.Given, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Importee_Given

object Importee_Given:
  val marker = 0 // assert: DumpStructure.structure
  object G:
    given Ordering[Int] = Ordering.Int
  import G.given Ordering[Int]
