// A quote's copy of a call the typer computed (a tuple's builtin member) names the copies of
// its receiver and arguments: of the tree put for a hole, of a receiver no copy holds but the
// record, never the quote's own nodes.
import scala.quoted.*

object CopiedCalls:
  transparent inline def size(x: (Int, String)): Int = ${ sizeImpl('x) }
  def sizeImpl(x: Expr[(Int, String)])(using Quotes): Expr[Int] = '{ $x.size }

  transparent inline def built(x: Int): Int = ${ builtImpl('x) }
  def builtImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ ($x, "a").size + ($x, 2, 3).head }
