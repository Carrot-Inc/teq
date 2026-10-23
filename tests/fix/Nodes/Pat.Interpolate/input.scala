/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Interpolate, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Interpolate

object Pat_Interpolate:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: String) = x match
    case s"a${y}b" => y
    case _ => ""
