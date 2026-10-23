/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Apply, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Apply

object Type_Apply:
  val marker = 0 // assert: DumpStructure.structure
  type A = Map[Int, String]
  type B = Either[Int, List[String]]
