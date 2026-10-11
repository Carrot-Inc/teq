// An upstream's macros whose results stand at their splices by their roots alone (`Inliner.expandMacro`'s
// `withSpan(splicePos.span)`, the root an `Inlined` wrapper inside an expansion): an argument the macro hands
// back keeps the downstream's place under it, and a constant of a transparent macro is folded at its call.
package mpa

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

object Lib:
  inline def forwarded(inline x: Int): String = M.loc(M.ident(x))
  inline def quoted(inline x: Int): String = M.loc(M.wrap(x))
