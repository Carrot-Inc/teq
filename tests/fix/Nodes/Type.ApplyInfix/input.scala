/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.ApplyInfix, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_ApplyInfix

object Type_ApplyInfix:
  val marker = 0 // assert: DumpStructure.structure
  type A = Int Either String
  type B = Int & Any
  type C = Int | String
