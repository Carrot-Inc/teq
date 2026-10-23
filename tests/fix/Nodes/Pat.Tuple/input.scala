/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Tuple, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Tuple

object Pat_Tuple:
  val marker = 0 // assert: DumpStructure.structure
  val (a, b) = (1, 2)
  def f(x: (Int, Int)) = x match
    case (1, y) => y
