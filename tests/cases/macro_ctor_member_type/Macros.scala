package mlib
import scala.quoted.*

object Shapes:
  inline def params[T]: String = ${ paramsImpl[T] }
  def paramsImpl[T: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    val tpe = TypeRepr.of[T]
    val ctor = tpe.typeSymbol.primaryConstructor
    val shown = tpe.memberType(ctor) match
      case MethodType(names, types, _) => names.zip(types.map(_.typeSymbol.name)).map((n, t) => n + ": " + t).mkString(", ")
      case _ => "no method type"
    Expr(shown)

  inline def isA[T](inline x: Any): Boolean = ${ isAImpl[T]('x) }
  def isAImpl[T: Type](x: Expr[Any])(using q: Quotes): Expr[Boolean] =
    import q.reflect.*
    val bind = Symbol.newBind(Symbol.spliceOwner, "v", Flags.EmptyFlags, TypeRepr.of[T])
    val yes = CaseDef(Bind(bind, Typed(Wildcard(), TypeTree.of[T])), None, Literal(BooleanConstant(true)))
    val no = CaseDef(Wildcard(), None, Literal(BooleanConstant(false)))
    Match(x.asTerm, List(yes, no)).asExprOf[Boolean]
