import scala.quoted.*
inline def inspect(inline e: String): String = ${ inspectImpl('e) }
def inspectImpl(e: Expr[String])(using Quotes): Expr[String] =
  import quotes.reflect.*
  def loop(t: Term): String = t match
    case Inlined(_, _, e) => loop(e)
    case Block((v: ValDef) :: _, _) => v.tpt.tpe.show
    case _ => "no binding"
  Expr(loop(e.asTerm))
