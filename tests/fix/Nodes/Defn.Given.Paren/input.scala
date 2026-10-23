/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Givens of a parenthesized type, their form as scalameta 4.17.4's `givenDecl` decides it (structure.txt): a body or a
// constructor call after the type's parentheses a `Defn.Given`, its `Init` from the parenthesis; nothing after them
// a `Decl.GivenAnonymous`.
package nodes.Defn_Given_Paren

trait A
trait B
trait C
class D

object Defn_Given_Paren:
  val marker = 0 // assert: DumpStructure.structure
  given (A) with {}
  given (B)
  given C with {}
  given (D)()
