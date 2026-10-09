import scala.quoted.*

// The reflection API inside a context function of its own `Quotes`: its paths start from the
// closure's parameter, not from the enclosing method's.
object LambdaQuotes:
  def f(using q: Quotes): Quotes ?=> String =
    (r: Quotes) ?=>
      import r.reflect.*
      r.reflect.TypeRepr.of[Int].show

  inline def shown: String = ${ shownImpl }
  def shownImpl(using q: Quotes): Expr[String] = Expr("lambda: " + (f != null))
