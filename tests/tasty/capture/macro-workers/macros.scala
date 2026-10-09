// A quote typed by one worker runs in the macro expansions of the others: each expansion's copy
// takes the records of the quote's body however the files are shared among the workers.
import scala.quoted.*

object MacroWorkers:
  transparent inline def isString(x: Any): Boolean = ${ isStringImpl('x) }
  def isStringImpl(x: Expr[Any])(using Quotes): Expr[Boolean] = '{ $x.isInstanceOf[String] }
