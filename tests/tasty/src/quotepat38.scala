// Compiled with Scala 3.8.4 (TASTy 28.8), which pickles a quote pattern as a `QUOTEPATTERN`, its
// holes `SPLICEPATTERN`s and its type variables bindings after the pattern's type, where 3.3
// wrote an `unapply` of `QuoteMatching.ExprMatch` (src/quotepat.scala;
// `tests/classpath/js/quote_patterns_28_8.scala`).
package fix.qp38

import scala.quoted.*

object Qp38:
  def plain(n: Int): Int = n

  inline def isUnit(inline a: Any): Boolean = ${ isUnitImpl('a) }
  def isUnitImpl(a: Expr[Any])(using Quotes): Expr[Boolean] = a match
    case '{ () } => Expr(true)
    case _ => Expr(false)

  inline def kind(inline e: Any): String = ${ kindImpl('e) }
  def kindImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ ($x: Int) + ($y: Int) } => Expr("add")
    case '{ ($l: List[t]).head } => Expr("head of " + Type.show[t])
    case '{ type t <: AnyVal; Some($v: t) } => Expr("some " + Type.show[t])
    case '{ Qp38.plain($a) } => Expr("plain " + a.valueOrAbort)
    case _ => Expr("other")

  inline def typeKind[T]: String = ${ typeKindImpl[T] }
  def typeKindImpl[T: Type](using Quotes): Expr[String] = Type.of[T] match
    case '[List[e]] => Expr("list of " + Type.show[e])
    case '[Int] => Expr("int")
    case _ => Expr("other")
