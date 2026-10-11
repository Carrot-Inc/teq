// A macro's result stands at its splice by its root alone (`Inliner.expandMacro`'s
// `withSpan(splicePos.span)`), and inside an expansion that root is a wrapper the trees under it keep
// their places through: the `Inlined` of an argument the macro hands back (`Splicer`) and of a fresh
// quote (`PickleQuotes`), which `inlinedNormalizer` keeps, `enclosingInlineds` holding the macro's call.
// Reflection's position past the wrappers is the argument's and the quote's own; a constant result of
// a transparent macro is folded at its call. scalac prints the lines of the .expected file.
import scala.quoted.*
object M:
  transparent inline def ident(inline x: Int): Int = ${ pass('x) }
  def pass(x: Expr[Int])(using Quotes): Expr[Int] = x
  transparent inline def wrap(inline x: Int): Int = ${ wrapImpl('x) }
  def wrapImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ identity($x) }
  transparent inline def seven(inline x: Int): Int = ${ sevenImpl('x) }
  def sevenImpl(x: Expr[Int])(using Quotes): Expr[Int] = Expr(7)
  inline def loc(inline x: Int): String = ${ where('x) }
  def where(x: Expr[Int])(using Quotes): Expr[String] =
    import quotes.reflect.*
    def strip(t: Term): Term = t match
      case Inlined(_, _, b) => strip(b)
      case _ => t
    val p = strip(x.asTerm).pos
    Expr(p.sourceFile.name + ":" + p.startLine.toString + ":" + p.startColumn.toString)
