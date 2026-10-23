/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.With, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_With

object Type_With:
  val marker = 0 // assert: DumpStructure.structure
  trait A
  trait B
  type AB = A with B
  trait S:
    self: A with B =>
