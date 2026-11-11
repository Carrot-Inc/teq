import scala.quoted.*

object Conds:
  def cond(): true = { println("condition"); true }

object Macros:
  inline def keepsIf: Boolean = ${ keepsIfImpl }

  def keepsIfImpl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    def isIf(t: Term): Boolean = t match
      case Inlined(_, _, body) => isIf(body)
      case If(_, _, _) => true
      case _ => false
    val a: Expr[String] = '{ "a" }
    Expr(isIf('{ if Conds.cond() then $a else "" }.asTerm))

  inline def folded: String = ${ foldedImpl }

  def foldedImpl(using Quotes): Expr[String] =
    val a: Expr[String] = '{ "a" }
    '{ (if Conds.cond() then $a else "") + "b" }
