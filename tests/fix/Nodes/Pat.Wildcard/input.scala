/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Wildcard, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Wildcard

object Pat_Wildcard:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Int) = x match
    case _ => 0
