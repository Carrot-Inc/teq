package rounds

import scala.quoted.*

object Counting:
  inline def spot: Int = ${ spotImpl }
  def spotImpl(using q: Quotes): Expr[Int] = Expr(q.reflect.Position.ofMacroExpansion.startLine + 1)
  inline def seven: Int = ${ sevenImpl }
  def sevenImpl(using Quotes): Expr[Int] = Expr(7)
