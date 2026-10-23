/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.SplicedMacroPat, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_SplicedMacroPat
import scala.quoted.*

object Term_SplicedMacroPat:
  val marker = 0 // assert: DumpStructure.structure
  def f(e: Expr[Int])(using Quotes): Expr[Int] = e match
    case '{ ${x}: Int } => x
    case _ => e
