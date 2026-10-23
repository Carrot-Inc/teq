/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.PartialFunction, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_PartialFunction

object Term_PartialFunction:
  val marker = 0 // assert: DumpStructure.structure
  val f: PartialFunction[Int, Int] = { case 1 => 2; case x if x > 1 => 3 }
