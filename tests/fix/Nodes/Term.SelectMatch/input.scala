/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.SelectMatch, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_SelectMatch

object Term_SelectMatch:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Int) = x.match
    case 1 => 2
    case _ => 3
