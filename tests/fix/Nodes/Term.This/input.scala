/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.This, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_This

object Term_This:
  val marker = 0 // assert: DumpStructure.structure
  class C:
    val x = 1
    def f = this.x
    def g = C.this.x
