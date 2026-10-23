/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.New, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_New

object Term_New:
  val marker = 0 // assert: DumpStructure.structure
  class C(a: Int)(b: Int)
  class D[A](a: A)
  class E
  val c = new C(1)(2)
  val d = new D[Int](1)
  val e = new E
  val f = new E()
