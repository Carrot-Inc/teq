/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Decl.Type, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Decl_Type

object Decl_Type:
  val marker = 0 // assert: DumpStructure.structure
  trait T:
    type A
    type B <: AnyRef
    type C[X] >: Null
