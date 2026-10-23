/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Match, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Match

object Term_Match:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Int) = x match
    case 1 => "a"
    case _ => "b"
  inline def g(inline x: Int) = inline x match
    case 1 => 2
    case _ => 3
