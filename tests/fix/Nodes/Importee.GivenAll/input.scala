/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Importee.GivenAll, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Importee_GivenAll

object Importee_GivenAll:
  val marker = 0 // assert: DumpStructure.structure
  object G:
    given Ordering[Int] = Ordering.Int
  import G.given
