/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Member.ParamClauseGroup, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Member_ParamClauseGroup

object Member_ParamClauseGroup:
  val marker = 0 // assert: DumpStructure.structure
  def f[A](a: A)(b: Int) = a
  extension (x: Int) def g = x
