// The macro of the case, `Pos.here`, gives `Position.ofMacroExpansion` (as in
// macro_position_nested).
import scala.quoted.*
object Pos:
  inline def here: String = ${ hereImpl }
  def hereImpl(using q: Quotes): Expr[String] =
    import q.reflect.*
    val p = Position.ofMacroExpansion
    Expr(p.sourceFile.name + ":" + p.startLine + ":" + p.startColumn)
  inline def gen: String = ${ genImpl }
  def genImpl(using Quotes): Expr[String] = '{ "gen " + Pos.here }
  inline def file: String = ${ fileImpl }
  def fileImpl(using q: Quotes): Expr[String] =
    import q.reflect.*
    Expr(SourceFile.current.name)
