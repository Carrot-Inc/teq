/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Eta, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Eta

object Term_Eta:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Int) = x
  val g = f _
