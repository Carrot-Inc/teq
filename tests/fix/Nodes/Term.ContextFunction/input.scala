/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.ContextFunction, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_ContextFunction

object Term_ContextFunction:
  val marker = 0 // assert: DumpStructure.structure
  val f: Int ?=> Int = (x: Int) ?=> x
  val g: Int ?=> Int = x ?=> x
