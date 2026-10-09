// `Position.ofMacroExpansion` is the outermost inline call under expansion, as scalac's `Inliner`
// positions the expansion: a macro called directly reports its call, one an inline method of
// another file wraps reports the wrapper's call in the file being compiled, not the macro's call
// inside the wrapper, and a macro whose quote another macro's expansion brings reports that
// macro's call. `SourceFile.current` is the outermost call's file. scalac prints the lines of the
// .expected file.
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
