/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Bounds, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Bounds

object Type_Bounds:
  val marker = 0 // assert: DumpStructure.structure
  def f[A >: Null <: AnyRef](a: A) = a
  def g[A: Ordering](a: A) = a
  type T >: Int <: Any
  def h[A: Ordering: Numeric](a: A) = a
