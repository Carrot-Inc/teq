package mlib
import scala.quoted.*

object Calls:
  private def strip(using q: Quotes)(t: q.reflect.Term): q.reflect.Term =
    import q.reflect.*
    t match
      case Inlined(_, _, b) => strip(b)
      case Typed(e, _) => strip(e)
      case _ => t

  inline def describe(inline x: Any): String = ${ describeImpl('x) }
  def describeImpl(x: Expr[Any])(using q: Quotes): Expr[String] =
    import q.reflect.*
    val shown = strip(x.asTerm) match
      case Apply(TypeApply(Select(_, name), targs), args) =>
        val as = args.map {
          case Ident(n) => "ident " + n
          case Literal(c) => "literal " + c.value
          case _ => "other"
        }
        s"$name[${targs.map(_.tpe.show).mkString(", ")}](${as.mkString(", ")})"
      case _ => "no call"
    Expr(shown)

  inline def rebuild[A](inline x: A): A = ${ rebuildImpl('x) }
  def rebuildImpl[A: Type](x: Expr[A])(using q: Quotes): Expr[A] =
    import q.reflect.*
    strip(x.asTerm) match
      case Apply(TypeApply(Select(lhs, name), targs), args) =>
        Select.overloaded(lhs, name, targs.map(_.tpe), args).asExprOf[A]
      case Apply(Select(lhs, name), args) =>
        Select.overloaded(lhs, name, Nil, args).asExprOf[A]
      case other => report.errorAndAbort("no call")

  inline def widened(inline x: Any): Any = ${ widenedImpl('x) }
  def widenedImpl(x: Expr[Any])(using q: Quotes): Expr[Any] =
    import q.reflect.*
    strip(x.asTerm) match
      case Apply(TypeApply(Select(lhs, name), _), args) =>
        Select.overloaded(lhs, name, List(TypeRepr.of[Long]), args).asExpr
      case other => report.errorAndAbort("no call")
