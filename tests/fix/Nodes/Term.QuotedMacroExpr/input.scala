/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.QuotedMacroExpr, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_QuotedMacroExpr
import scala.quoted.*

object Term_QuotedMacroExpr:
  val marker = 0 // assert: DumpStructure.structure
  def f(using Quotes): Expr[Int] = '{ 1 + 1 }
