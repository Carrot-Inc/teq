/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Throw, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Throw

object Term_Throw:
  val marker = 0 // assert: DumpStructure.structure
  def f(): Int = throw new RuntimeException("x")
