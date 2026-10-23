/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Var, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Var

object Pat_Var:
  val marker = 0 // assert: DumpStructure.structure
  val Some(x) = Option(1): @unchecked
  def f(y: Int) = y match
    case z => z
