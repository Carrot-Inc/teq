/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Var, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Var

object Type_Var:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Any) = x match
    case xs: List[a] => 1
    case _ => 0
