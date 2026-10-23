/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Extract, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Extract

object Pat_Extract:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Option[Int]) = x match
    case Some(1) => 1
    case None => 0
    case _ => 2
