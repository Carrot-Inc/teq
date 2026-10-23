/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.While, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_While

object Term_While:
  val marker = 0 // assert: DumpStructure.structure
  var i = 0
  def f() = while i < 2 do i += 1
  def g() = while (i < 3) { i += 1 }
