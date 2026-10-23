/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Decl.Val, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Decl_Val

object Decl_Val:
  val marker = 0 // assert: DumpStructure.structure
  trait T:
    val x: Int
    val y, z: String
