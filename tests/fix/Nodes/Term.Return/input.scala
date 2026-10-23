/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Return, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Return

object Term_Return:
  val marker = 0 // assert: DumpStructure.structure
  def f(): Int = return 1
  def g(): Unit = return
