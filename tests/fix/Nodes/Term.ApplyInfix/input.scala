/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.ApplyInfix, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_ApplyInfix

object Term_ApplyInfix:
  val marker = 0 // assert: DumpStructure.structure
  val a = 1 + 2
  val b = List(1) :+ 2
  val c = 1 :: Nil
  val d = (1, 2) == (1, 2)
