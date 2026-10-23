/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Ctor.Primary, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Ctor_Primary

object Ctor_Primary:
  val marker = 0 // assert: DumpStructure.structure
  class A private[Ctor_Primary] (x: Int)(y: Int)
  class B protected (x: Int)
  class C(using Int)
  class D
