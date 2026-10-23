/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Apply, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Apply

object Term_Apply:
  val marker = 0 // assert: DumpStructure.structure
  def f(a: Int)(b: Int) = a
  val x = f(1)(2)
  val y = List(1, 2)
  val z = println()
