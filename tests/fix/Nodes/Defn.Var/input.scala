/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Var, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Var

object Defn_Var:
  val marker = 0 // assert: DumpStructure.structure
  var a = 1
  var b: Int = 2
  var c: Int = _
  private var d = 3
