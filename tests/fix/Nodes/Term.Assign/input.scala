/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Assign, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Assign

object Term_Assign:
  val marker = 0 // assert: DumpStructure.structure
  var x = 1
  def f() = x = 2
