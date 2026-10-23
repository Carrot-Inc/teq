/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Case, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Case

object Case:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Int) = x match
    case 1 if x > 0 => 2
    case y => y
