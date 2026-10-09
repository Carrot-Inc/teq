import scala.quoted.*
object Pos:
  inline def fail(inline b: Boolean): Unit = ${ failImpl('b) }
  def failImpl(b: Expr[Boolean])(using q: Quotes): Expr[Unit] =
    import q.reflect.*
    if b.valueOrAbort then report.error("boom at the expansion", Position.ofMacroExpansion)
    else report.error("boom")
    '{ () }
