/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Repeated, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Repeated

object Pat_Repeated:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: List[Int]) = x match
    case List(first, rest*) => rest
    case _ => Nil
