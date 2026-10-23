/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Refine, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Refine

object Type_Refine:
  val marker = 0 // assert: DumpStructure.structure
  trait A
  type R = A { def m: Int }
  type S = { type T; val v: Int }
