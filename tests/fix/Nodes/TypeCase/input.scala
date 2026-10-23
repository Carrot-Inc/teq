/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// TypeCase, its fields as scalameta 4.17.4's (structure.txt).
package nodes.TypeCase

object TypeCase:
  val marker = 0 // assert: DumpStructure.structure
  type M[X] = X match
    case Int => String
    case _ => Int
