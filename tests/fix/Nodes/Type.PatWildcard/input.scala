/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.PatWildcard, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_PatWildcard

object Type_PatWildcard:
  val marker = 0 // assert: DumpStructure.structure
  type M[X] = X match
    case List[_] => Int
    case _ => String
