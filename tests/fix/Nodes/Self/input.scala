/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Self, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Self

object Self:
  val marker = 0 // assert: DumpStructure.structure
  trait A
  trait S:
    self: A =>
  trait T:
    this: A =>
  trait V:
    self =>
