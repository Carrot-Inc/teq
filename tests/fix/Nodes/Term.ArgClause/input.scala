/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.ArgClause, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_ArgClause

object Term_ArgClause:
  val marker = 0 // assert: DumpStructure.structure
  def f(a: Int, b: Int)(using c: Int) = a
  given Int = 1
  val x = f(1, 2)(using 3)
  val y = f(1, 2)
