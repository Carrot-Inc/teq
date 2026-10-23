/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Placeholder: a placeholder and a var initialized to its default, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Placeholder

object Term_Placeholder:
  val marker = 0 // assert: DumpStructure.structure
  val a = List(1).map(_ * 2)
  var b: Int = _
