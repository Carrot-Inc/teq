/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Given, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Given

object Pat_Given:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Option[Int]) = x match
    case Some(given Int) => summon[Int]
    case _ => 0
