package meridian.core.optics

import scala.quoted.*

/** `Lens.gen[S](_.a.b)` from the field path of the lambda: the getter is the lambda itself and
  * the setter a chain of `copy` calls, one per level. */
object LensMacro:
  def gen[S: Type, A: Type](path: Expr[S => A])(using Quotes): Expr[Lens[S, A]] =
    import quotes.reflect.*

    def fieldsOf(term: Term, acc: List[String]): List[String] = term match
      case Select(qualifier, name) => fieldsOf(qualifier, name :: acc)
      case Ident(_) => acc
      case Inlined(_, _, inner) => fieldsOf(inner, acc)
      case Block(Nil, inner) => fieldsOf(inner, acc)
      case Typed(inner, _) => fieldsOf(inner, acc)
      case other => report.errorAndAbort(s"not a field path: ${other.show}")

    def bodyOf(term: Term): Term = term match
      case Inlined(_, _, inner) => bodyOf(inner)
      case Block(List(DefDef(_, _, _, Some(body))), _) => body
      case Lambda(_, body) => body
      case other => report.errorAndAbort(s"not a lambda: ${other.show}")

    val names = fieldsOf(bodyOf(path.asTerm), Nil)
    if names.isEmpty then report.errorAndAbort("an empty path")

    def setter(obj: Term, rest: List[String], value: Term): Term = rest match
      case Nil => value
      case name :: more =>
        val symbol = obj.tpe.widen.typeSymbol
        val args = symbol.caseFields.map { field =>
          if field.name == name then NamedArg(name, setter(Select(obj, field), more, value))
          else NamedArg(field.name, Select(obj, field))
        }
        Apply(Select.unique(obj, "copy"), args)

    '{ Lens[S, A]($path)((a: A) => (s: S) => ${ setter('s.asTerm, names, 'a.asTerm).asExprOf[S] }) }
