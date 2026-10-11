// A macro's quotes that apply an inline parameter, reduced where the expansion puts the lambda argument as
// dotty's `BetaReduce` reduces an application (`InlineTyper.typedApply`): a curried application at each of
// its lists (`f(1)(2)` is 3), an argument of a constant type standing for the parameter as its literal
// (`f(1 + 2)` is 6), an argument with an effect bound to a val once, before the closure's body runs.
import scala.quoted.*
object Q:
  inline def curried(inline f: Int => Int => Int): String = ${ showImpl('{ f(1)(2) }) }
  inline def folded(inline f: Int => Int): String = ${ showImpl('{ f(1 + 2) }) }
  inline def runCurried(inline f: Int => Int => Int): Int = ${ passImpl('{ f(1)(2) }) }
  inline def runFolded(inline f: Int => Int): Int = ${ passImpl('{ f(1 + 2) }) }
  inline def runEffect(inline f: Int => Int): Int = ${ passImpl('{ f({ println("a"); 1 }) }) }
  def showImpl(e: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(e.asTerm.show)
  def passImpl(e: Expr[Int])(using Quotes): Expr[Int] = e
