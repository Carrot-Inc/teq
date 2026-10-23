/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Super, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Super

object Term_Super:
  val marker = 0 // assert: DumpStructure.structure
  class B:
    def f = 1
  trait T:
    def f = 2
  class C extends B with T:
    override def f = super.f + super[B].f + C.this.hashCode
  class E extends B with T:
    override def f = super[/* mix */ B].f
