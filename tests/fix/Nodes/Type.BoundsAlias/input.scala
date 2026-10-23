/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.BoundsAlias, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_BoundsAlias

object Type_BoundsAlias:
  val marker = 0 // assert: DumpStructure.structure
  def f[A: Ordering as ord](a: A) = ord.compare(a, a)
  def g[A: {Ordering as o, Numeric as n}](a: A) = a
