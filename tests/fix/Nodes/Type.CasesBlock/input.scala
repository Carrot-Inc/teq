/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.CasesBlock, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_CasesBlock

object Type_CasesBlock:
  val marker = 0 // assert: DumpStructure.structure
  type M[X] = X match
    case Int => String
    case _ => Int
  type N[X] = X match { case Int => String }
