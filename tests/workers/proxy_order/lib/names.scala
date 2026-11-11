package lib

import scala.quoted.*

// A transparent macro whose expansion is the name of the reference it is given: an argument that is
// not a path is bound to a proxy first (`m$proxyN`), whose number the unit's fresh names give.
object Names:
  transparent inline def bound(m: Int): String = ${ named('m) }

  def named(x: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def name(t: Term): String = t match
      case Inlined(_, _, e) => name(e)
      case _ => t.symbol.name
    Expr(name(x.asTerm))
