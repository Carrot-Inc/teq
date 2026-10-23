/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.TypedParam, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_TypedParam

object Type_TypedParam:
  val marker = 0 // assert: DumpStructure.structure
  type F = (x: Int) => x.type
  type N = (a: Int, b: String)
  type Q = (`a b`: Int) => String
