import scala.quoted.*

object Tracer:
  inline given here: String = ${ hereImpl }
  def hereImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr("line " + (Position.ofMacroExpansion.startLine + 1))
