/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.AnonymousFunction, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_AnonymousFunction

object Term_AnonymousFunction:
  val marker = 0 // assert: DumpStructure.structure
  val a = List(1).map(_ + 1)
  val b: (Int, Int) => Int = _ + _
