import scala.quoted.*

object LocalQuote:
  inline def keep[A](inline value: A): A = ${ keepImpl[A]('value) }
  def keepImpl[A: Type](value: Expr[A])(using Quotes): Expr[A] =
    '{
      val local: A = $value
      ${ '{ local }.asExprOf[A] }
    }
