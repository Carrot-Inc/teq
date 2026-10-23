/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.ParamClause, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_ParamClause

object Term_ParamClause:
  val marker = 0 // assert: DumpStructure.structure
  def f(a: Int)(using b: Int)(implicit c: String) = a
  def g(using Int, String) = 1
