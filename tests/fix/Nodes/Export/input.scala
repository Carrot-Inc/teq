/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Export, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Export

object Export:
  val marker = 0 // assert: DumpStructure.structure
  object Inner:
    def f = 1
  export Inner.f
