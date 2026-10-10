// An upstream's macro whose quotes apply an inline parameter (`'{ f(1) }`): the downstream's expansion
// reduces the application of the lambda argument it puts there (`InlineTyper.typedApply`'s `BetaReduce`,
// then `constToLiteral`), the other quotes keeping the parameter's constant path and the typed argument;
// a curried application reduces at each list, an argument of a constant type is its literal and an
// effect is bound once before the closure's body.
package iqa

import scala.quoted.*
import scala.compiletime.{codeOf, constValue}

object Q:
  inline def value(x: Int, inline f: Int => Int): Int = ${ valueImpl('{ f(1) }, '{ constValue[x.type] }) }
  def valueImpl(f: Expr[Int], x: Expr[Int])(using Quotes): Expr[Int] = x
  inline def code(inline x: Int, inline f: Int => Int): String = ${ codeImpl('{ f(1) }, '{ codeOf(x) }) }
  def codeImpl(f: Expr[Int], x: Expr[String])(using Quotes): Expr[String] = x
  inline def shape(inline f: Int => Int): String = ${ shapeImpl('{ f(1) }) }
  def shapeImpl(f: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(f.asTerm.show)
  inline def curried(inline f: Int => Int => Int): String = ${ shapeImpl('{ f(1)(2) }) }
  inline def folded(inline f: Int => Int): String = ${ shapeImpl('{ f(1 + 2) }) }
  inline def runEffect(inline f: Int => Int): Int = ${ passImpl('{ f({ println("a"); 1 }) }) }
  def passImpl(e: Expr[Int])(using Quotes): Expr[Int] = e
