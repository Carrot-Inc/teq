/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Function, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Function

object Type_Function:
  val marker = 0 // assert: DumpStructure.structure
  type A = Int => Int
  type B = (Int, Int) => Int
  type C = Int => Int => Int
