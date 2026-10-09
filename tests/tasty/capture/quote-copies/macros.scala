// A quote's copy takes the records of its patterns and of its locals: the extractor of a
// sequence pattern, the types of the temporaries named arguments are bound to.
import scala.quoted.*

object QuoteCopies:
  def g(a: Int, b: Int): Int = a - b
  def side(i: Int): Int = i

  transparent inline def sum2(xs: List[Int]): Int = ${ sum2Impl('xs) }
  def sum2Impl(xs: Expr[List[Int]])(using Quotes): Expr[Int] =
    '{ $xs match { case List(a, b) => a + b; case _ => 0 } }

  transparent inline def named(): Int = ${ namedImpl }
  def namedImpl(using Quotes): Expr[Int] = '{ g(b = side(2), a = side(1)) }
