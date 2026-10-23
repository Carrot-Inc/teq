/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Function, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Function

object Term_Function:
  val marker = 0 // assert: DumpStructure.structure
  val a = (x: Int) => x
  val b: Int => Int = x => x
  val c = (x: Int, y: Int) => x + y
  val d = () => 1
