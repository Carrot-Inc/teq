// A macro's quote applying an inline lambda parameter to constants, reduced as dotty's `BetaReduce` reduces it: the
// parameters substituted by `TreeTypeMap`, whose typed copier types each operation copied over constant operands
// again (`TypedTreeCopier.Apply`'s `assignType`, `ConstFold`), so an operation under one that folds nothing (a
// condition, a selection) is its literal, over the operands as written (`x + 2L` is `Int.+(Long)`, no conversion of
// `x`); the reduction's top folded by `constToLiteral`.
import scala.quoted.*
object Q:
  inline def folded(inline f: Int => Int): String = ${ showImpl('{ f(1 + 2) }) }
  inline def folded2(inline f: (Int, Int) => Int): String = ${ showImpl('{ f(1, 2) }) }
  inline def foldedS(inline f: (String, String) => Int): String = ${ showImpl('{ f("a", "b") }) }
  inline def foldedL(inline f: Int => Long): String = ${ showAnyImpl('{ f(3) }) }
  inline def foldedB(inline f: Int => Boolean): String = ${ showAnyImpl('{ f(3) }) }
  inline def run2(inline f: (Int, Int) => Int): Int = ${ passImpl('{ f(1, 2) }) }
  def showImpl(e: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(e.asTerm.show)
  def showAnyImpl[T: Type](e: Expr[T])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(e.asTerm.show)
  def passImpl(e: Expr[Int])(using Quotes): Expr[Int] = e
