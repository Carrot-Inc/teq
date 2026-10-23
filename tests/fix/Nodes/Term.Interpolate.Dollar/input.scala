/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's DollarValue probe, its fields, values, syntax and derived accessors as scalameta 4.17.4's (structure.txt).
package nodes.Term_Interpolate_Dollar

object O { val x = s"$$" }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
