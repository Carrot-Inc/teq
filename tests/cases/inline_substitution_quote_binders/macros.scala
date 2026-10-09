import scala.quoted.*

trait Marker
object O extends Marker

object M:
  inline def check(x: AnyRef): Boolean =
    ${ inspect('{ val y: x.type = x; y }) }

  def inspect(e: Expr[Any])(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    def inspectTerm(t: Term): Boolean = t match
      case Inlined(_, _, body) => inspectTerm(body)
      case Block(List(v: ValDef), _) =>
        v.tpt.tpe.widen <:< TypeRepr.of[Marker]
      case _ => false
    Expr(inspectTerm(e.asTerm))
