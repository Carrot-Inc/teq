import scala.quoted.*

inline def selected(inline value: Any): String = ${ selectedImpl('value) }
def selectedImpl(value: Expr[Any])(using Quotes): Expr[String] =
  import quotes.reflect.*
  def core(term: Term): Term = term match
    case Inlined(_, Nil, expansion) => core(expansion)
    case _ => term
  val shown = core(value.asTerm) match
    case Apply(fun, args) => s"${fun.symbol.name}(${fun.symbol.signature.paramSigs.mkString(", ")})"
    case _ => "other"
  Expr(shown)
