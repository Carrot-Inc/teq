/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Name.This, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Name_This

object Name_This:
  val marker = 0 // assert: DumpStructure.structure
  trait T:
    this: Any =>
    def f = 1
