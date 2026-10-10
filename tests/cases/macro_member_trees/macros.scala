package shown

import scala.quoted.*

object Shown:
  inline def show(inline x: Any): String = ${ showImpl('x) }
  def showImpl(x: Expr[Any])(using Quotes): Expr[String] = Expr(x.show)
  inline def structure(inline x: Any): String = ${ structureImpl('x) }
  def structureImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    // The argument under dotty's `Inlined` of an inline parameter, which teq does not wrap it in.
    val t = x.asTerm match
      case Inlined(_, Nil, t) => t
      case t => t
    Expr(t.show(using Printer.TreeStructure))
