/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Decl.Def, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Decl_Def

object Decl_Def:
  val marker = 0 // assert: DumpStructure.structure
  trait T:
    def f: Int
    def g[A](a: A)(using Int): A
