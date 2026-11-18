import scala.quoted.*

object Mac:
  inline def size(inline x: Any): Int = ${ sizeImpl('x) }
  def sizeImpl(x: Expr[Any])(using Quotes): Expr[Int] = Expr(x.show.length)
