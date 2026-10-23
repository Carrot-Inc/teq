/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's UnaryValue probe, its fields, values, syntax and derived accessors as scalameta 4.17.4's (structure.txt).
package nodes.Lit_WithUnary_Value

object O { val x = -'a' }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
