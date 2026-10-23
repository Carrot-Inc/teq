/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.SplicedMacroExpr, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_SplicedMacroExpr
import scala.quoted.*

object Term_SplicedMacroExpr:
  val marker = 0 // assert: DumpStructure.structure
  def f(e: Expr[Int])(using Quotes): Expr[Int] = '{ ${ e } + 1 }
