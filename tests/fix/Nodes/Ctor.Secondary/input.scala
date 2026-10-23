/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Ctor.Secondary, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Ctor_Secondary

object Ctor_Secondary:
  val marker = 0 // assert: DumpStructure.structure
  class C(x: Int):
    def this() = this(1)
    def this(s: String) =
      this(s.length)
      println()
