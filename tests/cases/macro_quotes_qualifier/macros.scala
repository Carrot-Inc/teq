import scala.quoted.*

// The reflection API's paths through the `Quotes` the source names where several are in
// scope: the given one an import and a declared type name, the explicit one unused and null,
// and an explicit one named the same ways; the pickle keeps each, as scalac's types do.
object M:
  inline def inspect(inline x: Int): String = ${ impl('x) }

  def classify(using q: Quotes)(r: Quotes)(t: q.reflect.Term): String =
    import q.reflect.*
    t match
      case i: Inlined => classify(using q)(r)(i.body)
      case l: Literal => "literal:" + l.constant.value
      case _ => "other"

  def named(using q: Quotes)(r: Quotes)(t: r.reflect.Term): String =
    import r.reflect.*
    t match
      case i: Inlined => named(using q)(r)(i.body)
      case l: Literal => "named:" + l.constant.value
      case _ => "other"

  def impl(x: Expr[Int])(using q: Quotes): Expr[String] =
    import q.reflect.*
    Expr(classify(using q)(null)(x.asTerm) + " " + named(using q)(q)(x.asTerm))
