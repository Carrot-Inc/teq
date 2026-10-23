/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Project, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Project

object Type_Project:
  val marker = 0 // assert: DumpStructure.structure
  class O:
    type I = Int
  type P = O#I
