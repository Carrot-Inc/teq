/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Def, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Def

object Defn_Def:
  val marker = 0 // assert: DumpStructure.structure
  def a = 1
  def b(x: Int): Int = x
  private inline def c[A](a: A)(using Int): A = a
  def d(): Unit = ()
