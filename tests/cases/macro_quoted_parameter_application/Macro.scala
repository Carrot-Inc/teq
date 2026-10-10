// A macro's quotes that apply an inline parameter (`'{ f(1) }`) beside ones that read another parameter's
// singleton (`'{ constValue[x.type] }`) and its code (`'{ codeOf(x) }`): the expansion puts each argument in
// place and reduces the application of a lambda argument (`InlineTyper.typedApply`'s `BetaReduce`), a
// constant it leaves folded to its literal (`constToLiteral`), so `shape` shows `2`, while the other quotes
// keep the parameter's constant path and the typed argument.
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
