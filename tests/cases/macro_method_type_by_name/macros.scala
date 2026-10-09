// The method type of a selected method with a by-name parameter: the parameter's type is a
// `ByNameType` over the type written, which `widenByName` and `widen` take off; by-name types
// compare by what they are over.
import scala.quoted.*

inline def paramKinds[T](inline call: T): String = ${ paramKindsImpl('call) }

def paramKindsImpl[T: Type](call: Expr[T])(using Quotes): Expr[String] =
  import quotes.reflect.*
  def fun(t: Term): Term = t match
    case Inlined(_, _, e) => fun(e)
    case Typed(e, _) => fun(e)
    case Apply(f, _) => f
    case other => other
  fun(call.asTerm).tpe.widen match
    case MethodType(_, params, _) =>
      val kinds = params.map {
        case b @ ByNameType(u) => s"byname ${u.show} ${b.widenByName =:= u}"
        case other => s"plain ${other.show}"
      }
      val first = params.head
      val made = ByNameType(TypeRepr.of[Int])
      val compared = List(
        made =:= ByNameType(TypeRepr.of[Int]),
        made <:< ByNameType(TypeRepr.of[Int]),
        first =:= made,
        first.widen =:= TypeRepr.of[Int],
        made =:= TypeRepr.of[Int],
        ByNameType(TypeRepr.of[String]) <:< ByNameType(TypeRepr.of[Any]),
      )
      Expr(kinds.mkString(", ") + " | " + compared.mkString(" "))
    case _ => Expr("no method type")
