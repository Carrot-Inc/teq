/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Val, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Val

object Defn_Val:
  val marker = 0 // assert: DumpStructure.structure
  val a = 1
  private lazy val b: Int = 2
  val (c, d) = (1, 2)
  val e, f = 3
