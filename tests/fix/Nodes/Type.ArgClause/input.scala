/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.ArgClause, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_ArgClause

object Type_ArgClause:
  val marker = 0 // assert: DumpStructure.structure
  type A = List[Int]
  val b = List.empty[String]
