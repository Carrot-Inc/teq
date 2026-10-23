/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// A dependent function type's parameter is a `Type.TypedParam`, and a refinement written alone is a `Type.Refine` of no
// base type over its `Stat.Block`, as scalameta's.
object TypeShapes:
  val marker = 0 // assert: DumpLayout.layout
  type F = (x: Int) => x.type
  type B = { def value: Int }
