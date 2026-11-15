package relays

import scala.quoted.*

object Probe:
  inline def probe(inline n: Int): String = ${ probeCode('n) }
  def probeCode(n: Expr[Int])(using Quotes): Expr[String] = Expr(Relay.total(n.valueOrAbort))
