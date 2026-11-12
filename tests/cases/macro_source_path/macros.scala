import scala.quoted.*

// What a macro reads of its expansion's source file: the path the compiler was given, relative
// where it was, as `path`, `jpath` and `getJPath` (dotty's `QuotesImpl.SourceFileMethods`, the
// source's own path), and the file's name. tests/split.sh expects a
// build from two places of one checkout to write the same output.
object SourcePath:
  inline def here: String = ${ hereImpl }
  def hereImpl(using Quotes): Expr[String] =
    val f = quotes.reflect.Position.ofMacroExpansion.sourceFile
    Expr(s"${f.path} | ${f.getJPath.map(_.toString)} | ${f.jpath.isAbsolute} | ${f.name}")
