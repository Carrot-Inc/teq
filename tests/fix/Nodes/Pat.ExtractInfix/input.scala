/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.ExtractInfix, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_ExtractInfix

object Pat_ExtractInfix:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: List[Int]) = x match
    case h :: t => h
    case _ => 0
