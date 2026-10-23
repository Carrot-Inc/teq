/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Lambda, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Lambda

object Type_Lambda:
  val marker = 0 // assert: DumpStructure.structure
  type L = [X] =>> List[X]
  type M = [X, Y] =>> Map[X, Y]
