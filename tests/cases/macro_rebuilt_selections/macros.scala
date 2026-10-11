import scala.quoted.*

// What scalactic's `BooleanMacro` and `RequirementsMacro` read of their arguments and rebuild.
object M:
  // A vararg's argument as dotty types it, `Typed(Repeated(args, elemtpt), T*)`, seen through the
  // parameter's binding (`underlyingArgument`, `MapToUnderlying`).
  inline def shape(args: Any*): String = ${ shapeImpl('args) }
  def shapeImpl(args: Expr[Seq[Any]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(args.asTerm.underlyingArgument match
      case Typed(Repeated(xs, _), _) => s"Typed(Repeated(${xs.map(_.show).mkString(", ")}))"
      case _ => "another tree")

  // A comparison rebuilt on a new receiver by the operator's symbol: `left.select(sel.symbol)`.
  inline def rebuilt(inline c: Boolean): String = ${ rebuiltImpl('c) }
  def rebuiltImpl(c: Expr[Boolean])(using Quotes): Expr[String] =
    import quotes.reflect.*
    c.asTerm.underlyingArgument match
      case Apply(sel @ Select(lhs, op), rhs :: Nil) =>
        val again = ValDef.let(Symbol.spliceOwner, lhs) { left => left.select(sel.symbol).appliedTo(rhs) }.asExprOf[Boolean]
        '{ ${ Expr(op + " " + sel.symbol.name) } + " " + $again }
      case _ => Expr("another tree")
