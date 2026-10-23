/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.NewAnonymous, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_NewAnonymous

object Term_NewAnonymous:
  val marker = 0 // assert: DumpStructure.structure
  class C(a: Int)
  trait T
  val a = new C(1) { val y = 2 }
  val b = new C(1) with T {}
  val c = new T {}
