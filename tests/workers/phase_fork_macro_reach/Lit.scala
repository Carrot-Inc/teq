import scala.quoted.*

object Lit:
  extension (inline sc: StringContext)
    inline def code(inline args: Any*): Model.Code = ${ impl('sc) }

  def impl(sc: Expr[StringContext])(using Quotes): Expr[Model.Code] =
    val s = sc.valueOrAbort.parts.head
    Model.Code.parse(s) match
      case Some(c) => '{ Model.Code(${ Expr(c.value) }) }
      case None => quotes.reflect.report.errorAndAbort("not a code: " + s)
