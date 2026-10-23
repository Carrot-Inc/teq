/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Name, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Name

object Term_Name:
  val marker = 0 // assert: DumpStructure.structure
  val x = 1
  val `a b` = 2
  val y = x + `a b`
  val `type` = 3
