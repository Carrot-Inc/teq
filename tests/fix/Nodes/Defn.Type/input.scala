/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Type, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Type

object Defn_Type:
  val marker = 0 // assert: DumpStructure.structure
  type A = Int
  opaque type B = Int
  type C[X] = List[X]
  type D >: Null <: AnyRef
