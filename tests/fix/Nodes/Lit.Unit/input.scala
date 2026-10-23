/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Lit.Unit, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Lit_Unit

object Lit_Unit:
  val marker = 0 // assert: DumpStructure.structure
  val u = ()
  def f(c: Boolean): Unit = if c then println(1)
