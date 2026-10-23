/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Decl.Given, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Decl_Given

object Decl_Given:
  val marker = 0 // assert: DumpStructure.structure
  trait T:
    given x: Int
    given ord: Ordering[Int]
