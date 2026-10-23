/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Alternative, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Alternative

object Pat_Alternative:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Int) = x match
    case 1 | 2 | 3 => 1
    case _ => 0
