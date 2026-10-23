/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Name, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Name

object Type_Name:
  val marker = 0 // assert: DumpStructure.structure
  type A = Int
  type `a b` = String
  val x: `a b` = ""
