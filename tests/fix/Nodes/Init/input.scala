/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Init, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Init

object Init:
  val marker = 0 // assert: DumpStructure.structure
  class B(x: Int)(y: Int)
  trait T
  class C extends B(1)(2) with T
  class D extends B(1)(2), T
