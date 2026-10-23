/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.QuotedMacroType, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_QuotedMacroType
import scala.quoted.*

object Term_QuotedMacroType:
  val marker = 0 // assert: DumpStructure.structure
  def f[T](t: Type[T])(using Quotes): Int = t match
    case '[List[a]] => 1
    case _ => 0
