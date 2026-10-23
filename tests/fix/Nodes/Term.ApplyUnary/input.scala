/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.ApplyUnary, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_ApplyUnary

object Term_ApplyUnary:
  val marker = 0 // assert: DumpStructure.structure
  val x = 1
  val a = -x
  val b = !true == false
  val c = ~x
  val d = +x
