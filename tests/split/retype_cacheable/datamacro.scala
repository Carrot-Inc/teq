package cacheable

import scala.quoted.*

/** Reads `data.txt` beside the file under expansion: a file a macro reads is read again in
  * every build. */
object DataMacro:
  def dataImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val source = Position.ofMacroExpansion.sourceFile.getJPath.get
    val file = source.getParent.resolve("data.txt")
    Expr(java.nio.file.Files.readString(file).trim)

  /** Reads `data.txt` as `dataImpl` does, then runs out of its call depth when `deep` is true. */
  def deepImpl(deep: Expr[Boolean])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val source = Position.ofMacroExpansion.sourceFile.getJPath.get
    val text = java.nio.file.Files.readString(source.getParent.resolve("data.txt")).trim
    if deep.valueOrAbort then Expr(text + Macros.deeper(0))
    else
      report.warning("read " + text)
      Expr(text)

inline def data: String = ${ DataMacro.dataImpl }
inline def readDeep(inline deep: Boolean): String = ${ DataMacro.deepImpl('deep) }
