/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's VarValue probe, its fields, values, syntax and derived accessors as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Var_Pattern

object O { var (a, b) = (1, 2) }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
