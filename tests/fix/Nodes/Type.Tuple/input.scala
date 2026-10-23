/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Tuple, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Tuple

object Type_Tuple:
  val marker = 0 // assert: DumpStructure.structure
  type A = (Int, String)
  type B = (Int, (String, Boolean))
