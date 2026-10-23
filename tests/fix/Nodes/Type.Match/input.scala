/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Match, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Match

object Type_Match:
  val marker = 0 // assert: DumpStructure.structure
  type M[X] = X match
    case Int => String
    case String => Int
