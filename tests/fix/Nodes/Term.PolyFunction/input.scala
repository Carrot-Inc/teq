/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.PolyFunction, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_PolyFunction

object Term_PolyFunction:
  val marker = 0 // assert: DumpStructure.structure
  val f = [A] => (a: A) => a
