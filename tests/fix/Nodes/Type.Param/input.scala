/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Param, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Param

object Type_Param:
  val marker = 0 // assert: DumpStructure.structure
  class C[+A, -B, D[_], E <: AnyRef]
  def f[A, F[_]](a: F[A]) = a
  type T[X] = List[X]
