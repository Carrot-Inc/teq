/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.FuncParamClause, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_FuncParamClause

object Type_FuncParamClause:
  val marker = 0 // assert: DumpStructure.structure
  type A = (Int, String) => Int
  type B = () => Int
  type C = Int => Int
