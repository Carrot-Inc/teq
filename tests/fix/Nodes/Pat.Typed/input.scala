/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Typed, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Typed

object Pat_Typed:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Any) = x match
    case i: Int => i
    case _: String => 0
    case _ => 1
