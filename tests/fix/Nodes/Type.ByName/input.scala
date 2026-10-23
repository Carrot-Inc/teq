/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.ByName, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_ByName

object Type_ByName:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: => Int) = x
  type F = (=> Int) => Int
