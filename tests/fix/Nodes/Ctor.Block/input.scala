/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Ctor.Block, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Ctor_Block

object Ctor_Block:
  val marker = 0 // assert: DumpStructure.structure
  class C(x: Int):
    def this() = { this(1); println() }
