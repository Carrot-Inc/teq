/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Annotate, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Annotate

object Term_Annotate:
  val marker = 0 // assert: DumpStructure.structure
  val x = 1
  val a = (x: @unchecked)
  val b = (x: @unchecked @deprecated)
