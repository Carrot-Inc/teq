// Compiled with Scala 3.8.4: the reflection API pickled through a `Quotes` value's `reflect`
// (`q.reflect.TypeReprMethods.typeSymbol(t)`, types `q.reflect.Symbol`), type tests as the
// `TypeTest` given's extractor (`q.reflect.LiteralTypeTest(l)`), `quotes.reflect` as the
// expansion of `Quotes$package.quotes`, and a splice nested in a quote, its `Quotes` its own.
package fix.r38

import scala.quoted.*

object Reflect38:
  inline def fields[T]: List[String] = ${ fieldsImpl[T] }
  def fieldsImpl[T: Type](using q: Quotes): Expr[List[String]] =
    import q.reflect.*
    val sym = TypeRepr.of[T].typeSymbol
    Expr(sym.caseFields.map(_.name))

  def isApply(using q: Quotes)(t: q.reflect.Term): Boolean =
    import q.reflect.*
    t match
      case Apply(_, _) => true
      case _ => false

  def kindOf(using Quotes)(t: quotes.reflect.Tree): String =
    import quotes.reflect.*
    t match
      case l: Literal => "lit " + l.constant.value
      case i: Ident => "ident " + i.name
      case _ => "other"

  def warn(e: Expr[Any])(using Quotes): Unit =
    quotes.reflect.report.warning("w", e)

  def nested(x: Expr[Int])(using Quotes): Expr[Int] =
    '{ val y = $x; ${ tenfold('y) } + 1 }
  def tenfold(y: Expr[Int])(using Quotes): Expr[Int] = '{ $y * 10 }

  inline def shape(inline e: Any): String = ${ shapeImpl('e) }
  def shapeImpl(e: Expr[Any])(using q: Quotes): Expr[String] =
    import q.reflect.*
    def go(t: Term): String = t match
      case Inlined(_, _, body) => go(body)
      case Apply(Select(recv, name), args) => s"${go(recv)}.$name(${args.size})"
      case Literal(c) => c.value.toString
      case Ident(n) => n
      case _ => "?"
    Expr(go(e.asTerm))
