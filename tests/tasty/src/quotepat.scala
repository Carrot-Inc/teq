// Compiled with Scala 3.3.8 (TASTy 28.3), which pickles an expression quote pattern as an
// `unapply` of `QuoteMatching.ExprMatch` (`tests/classpath/js/quote_patterns_28_3.scala`).
package fix.qpat

import scala.quoted.*

final class LayerBox[-I, +E, +O](val name: String)

object LayerMacros:
  inline def describe[E](inline b: LayerBox[?, E, ?]): String = ${ describeImpl[E]('b) }

  def describeImpl[E: Type](b: Expr[LayerBox[?, E, ?]])(using Quotes): Expr[String] =
    b match
      case '{ $x: LayerBox[i, e & E, o] } => Expr(s"${Type.show[i]} ${Type.show[e]} ${Type.show[o]}")

  inline def chain(inline a: LayerBox[?, ?, ?], inline b: LayerBox[?, ?, ?]): String = ${ chainImpl('a, 'b) }

  def chainImpl(a: Expr[LayerBox[?, ?, ?]], b: Expr[LayerBox[?, ?, ?]])(using Quotes): Expr[String] =
    a match
      case '{ $l: LayerBox[i, e, o] } =>
        b match
          case '{ $r: LayerBox[`o`, e2, o2] } => Expr(s"chained to ${Type.show[o2]}")
          case _ => Expr("unrelated")

  inline def isUnit(inline a: Any): Boolean = ${ isUnitImpl('a) }

  def isUnitImpl(a: Expr[Any])(using Quotes): Expr[Boolean] =
    a match
      case '{ () } => Expr(true)
      case _ => Expr(false)
