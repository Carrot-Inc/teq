/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Wildcard, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Wildcard

object Type_Wildcard:
  val marker = 0 // assert: DumpStructure.structure
  val a: List[?] = Nil
  val b: List[? <: AnyRef] = Nil
  def c(x: List[? >: Null <: AnyRef]) = x
  val d: List[_] = Nil
