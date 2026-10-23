/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.ParamClause, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_ParamClause

object Type_ParamClause:
  val marker = 0 // assert: DumpStructure.structure
  def f[A, B](a: A, b: B) = a
  class C[A]
  def g(x: Int) = x
