/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.ArgClause, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_ArgClause

object Pat_ArgClause:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Option[Int]) = x match
    case Some(y) => y
    case _ => 0
