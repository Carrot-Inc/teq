/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.If, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_If

object Term_If:
  val marker = 0 // assert: DumpStructure.structure
  def f(c: Boolean) = if c then 1 else 2
  def g(c: Boolean) = if (c) 1 else 2
  inline def h(inline c: Boolean) = inline if c then 1 else 2
  def k(c: Boolean): Unit = if c then println()
