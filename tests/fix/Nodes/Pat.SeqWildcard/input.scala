/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.SeqWildcard, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_SeqWildcard

object Pat_SeqWildcard:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: List[Int]) = x match
    case List(1, _*) => 1
    case _ => 0
