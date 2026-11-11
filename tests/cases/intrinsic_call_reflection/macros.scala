import scala.quoted.*

inline def inspect(inline value: Any): String = ${ inspectImpl('value) }
def inspectImpl(value: Expr[Any])(using Quotes): Expr[String] =
  import quotes.reflect.*
  def core(term: Term): Term = term match
    case Inlined(_, Nil, expansion) => core(expansion)
    case _ => term
  def description(term: Term): String = core(term) match
    case Apply(fun, args) => s"Apply:${fun.symbol.name}:${args.size}"
    case select: Select => s"Select:${select.symbol.name}"
    case _ => "other"
  val tree = core(value.asTerm)
  val copied = tree match
    case app @ Apply(fun, args) => Apply.copy(app)(fun, args)
    case sel @ Select(qual, name) => Select.copy(sel)(qual, name)
    case _ => tree
  Expr(description(tree) + "/" + description(copied))
