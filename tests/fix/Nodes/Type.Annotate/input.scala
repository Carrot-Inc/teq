/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Annotate, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Annotate

object Type_Annotate:
  val marker = 0 // assert: DumpStructure.structure
  type A = Int @unchecked
  val x: List[Int @unchecked] = Nil
