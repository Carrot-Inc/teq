package crates

import scala.quoted.*

object Probe:
  inline def probe(inline n: Int): String = ${ probeCode('n) }
  def probeCode(n: Expr[Int])(using Quotes): Expr[String] = Expr(Dock.total(n.valueOrAbort))
