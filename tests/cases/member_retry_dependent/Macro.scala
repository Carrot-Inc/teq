import scala.quoted.*

// Counters of their expansions on one count: a transparent number, and a transparent identity whose
// result type is its parameter's singleton.
object M:
  var n = 0
  transparent inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
  transparent inline def same(x: Int): x.type = ${ sameImpl[x.type]('x) }
  def sameImpl[T: Type](x: Expr[Int])(using Quotes): Expr[T] =
    n += 1
    '{ $x.asInstanceOf[T] }
