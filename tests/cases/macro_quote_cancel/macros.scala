import scala.quoted.*

// A quote of its splice alone, which scalac's typer cancels to the splice's code, and the
// reflection API inside a splice of a quote whose method names its `Quotes`: the splice's own
// `Quotes` qualifies both the type and the extension.
object Cancel:
  inline def same(inline x: Int): Int = ${ sameImpl('x) }
  def sameImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x }

  inline def typeName: String = ${ typeNameImpl }
  def typeNameImpl(using q: Quotes): Expr[String] =
    '{ "type: " + ${ Expr(quotes.reflect.TypeRepr.of[Int].show) } }

  inline def nested: Int = ${ nestedImpl }
  def nestedImpl(using Quotes): Expr[Int] = '{ ${ '{ ${ Expr(1) } } } }
