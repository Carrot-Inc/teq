/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Decl.Var, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Decl_Var

object Decl_Var:
  val marker = 0 // assert: DumpStructure.structure
  trait T:
    var x: Int
