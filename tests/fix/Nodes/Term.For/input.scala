/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.For, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_For

object Term_For:
  val marker = 0 // assert: DumpStructure.structure
  def f() = for x <- List(1) do println(x)
  def g() = for (x <- List(1)) println(x)
