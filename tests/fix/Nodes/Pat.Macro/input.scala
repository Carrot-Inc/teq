/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Macro, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Macro
import scala.quoted.*

object Pat_Macro:
  val marker = 0 // assert: DumpStructure.structure
  def f(e: Expr[Int])(using Quotes): Expr[Int] = e match
    case '{ ($x: Int) + 1 } => x
    case _ => e
