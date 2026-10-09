import scala.quoted.*

// A macro sees offsets and columns in UTF-16 units of the file's content, as scalac gives them.
object Pos:
  inline def here: String = ${ hereImpl }
  def hereImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val p = Position.ofMacroExpansion
    val content = p.sourceFile.content.getOrElse("")
    val text = content.substring(p.start, p.end)
    val back = Position(p.sourceFile, p.start, p.end)
    Expr(s"${p.start} ${p.end} ${p.startColumn} ${p.endColumn} ${p.startLine} [$text] [${back.sourceCode.getOrElse("")}] ${back.startColumn}")

  // A position between the two units of a pair keeps its unit boundaries, and its source is the
  // half it covers.
  inline def inPair: String = ${ inPairImpl }
  def inPairImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val f = Position.ofMacroExpansion.sourceFile
    val i = f.content.get.indexOf("\uD834\uDD1E") + 1
    val halves = List(Position(f, i, i + 1), Position(f, i - 1, i), Position(f, i - 1, i + 1), Position(f, i, i))
    Expr(halves.map(p => s"${p.start - i},${p.end - i},${p.sourceCode.getOrElse("").map(_.toInt).mkString("/")},${p.startColumn - Position(f, i - 1, i - 1).startColumn}").mkString(" "))

