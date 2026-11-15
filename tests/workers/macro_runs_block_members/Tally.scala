package vaults

import scala.quoted.*

object Tally:
  inline def tally(inline n: Int): Int = ${ tallyCode('n) }
  def tallyCode(n: Expr[Int])(using Quotes): Expr[Int] = Expr(Vault.total(n.valueOrAbort))
