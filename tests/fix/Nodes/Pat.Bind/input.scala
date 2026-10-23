/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Bind, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Bind

object Pat_Bind:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Option[Int]) = x match
    case s @ Some(_) => s
    case n @ None => n
