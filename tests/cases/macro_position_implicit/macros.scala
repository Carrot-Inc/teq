// `Position.ofMacroExpansion` of an inline given a search inferred: the end of the call it
// completes where that call stands in the file being compiled, the outermost inline call's
// whole span where the search is made inside an expansion, the code a macro's quote gives or the
// body of an inline method alike (scalac 3.8.4, `Inliner`). scalac prints the lines of the
// .expected file.
import scala.quoted.*
object P:
  case class Here(s: String)
  inline given here: Here = ${ hereImpl }
  def hereImpl(using q: Quotes): Expr[Here] =
    import q.reflect.*
    val p = Position.ofMacroExpansion
    val s = p.sourceFile.name + ":" + (p.startLine + 1) + ":" + p.startColumn + "-" + (p.endLine + 1) + ":" + p.endColumn
    '{ Here(${ Expr(s) }) }
  def takes(x: Int)(using h: Here): String = h.s
  extension (x: Int) inline def genOn: String = ${ genImpl('x) }
  def genImpl(x: Expr[Int])(using Quotes): Expr[String] = '{ P.takes($x) }
  inline def wrapped(x: Int): String = takes(x)
