// Inside a quote, `summonInline` and an inline given that a using clause resolves to stay calls
// of the quoted code: they expand where the quote is spliced, as scalac's inliner leaves
// level-1 code alone.
import scala.quoted.*
import scala.compiletime.summonInline

trait Show[A]:
  def show(a: A): String

def traced(using where: String): String = where

object Macros:
  inline def showIt[A](a: A): String = ${ showItImpl('a) }
  def showItImpl[A: Type](a: Expr[A])(using Quotes): Expr[String] =
    import Tracer.here
    '{ summonInline[Show[A]].show($a) + " at " + traced }
