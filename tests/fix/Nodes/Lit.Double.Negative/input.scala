/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's DecimalValue probe, its fields, values, syntax and derived accessors as scalameta 4.17.4's (structure.txt).
package nodes.Lit_Double_Negative

object O { val x = -1e-7 }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
