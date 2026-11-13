import scala.quoted.*

object Quoted:
  def same[T: Type](x: Expr[T])(using Quotes): Expr[T] = x
