/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Repeated, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Repeated

object Type_Repeated:
  val marker = 0 // assert: DumpStructure.structure
  def f(xs: Int*) = xs
  def g(xs: => Int*) = 1
