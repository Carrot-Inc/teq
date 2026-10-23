/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Mod.Implicit, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Implicit

object Mod_Implicit:
  val marker = 0 // assert: DumpStructure.structure
  implicit val a: Int = 1
  def f(implicit b: Int) = b
  implicit class C(x: Int)
