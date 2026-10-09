// The prefixes of named types as scalac shapes them: a package is a TermRef over the path of
// its parents down to the root's ThisType, the empty package a ThisType, a symbol's termRef and
// typeRef over its owner's ThisType; std classes by their scala-library full names.
package pp.inner

import scala.quoted.*

class Local
trait Svc[A]

object Macros:
  inline def describe[T]: String = ${ describeImpl[T] }
  inline def fullNames[T]: String = ${ fullNamesImpl[T] }

  def fullNamesImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    def names(t: TypeRepr): String = t.dealias match
      case AppliedType(f, args) => s"${names(f)}[${args.map(names).mkString(", ")}]"
      case other => other.typeSymbol.fullName
    Expr(names(TypeRepr.of[T]))

  def describeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    def kind(t: TypeRepr): String = t match
      case _: TermRef => "TermRef"
      case _: TypeRef => "TypeRef"
      case _: ThisType => "ThisType"
      case _: NoPrefix => "NoPrefix"
      case _: AppliedType => "AppliedType"
      case _ => "other"
    def chain(t: TypeRepr, depth: Int): String =
      if depth > 6 then "..." else t match
        case NoPrefix() => "NoPrefix"
        case TypeRef(q, n) => s"TypeRef[$n](${chain(q, depth + 1)})"
        case TermRef(q, n) => s"TermRef[$n](${chain(q, depth + 1)})"
        case ThisType(tr) => s"ThisType(${chain(tr, depth + 1)})"
        case AppliedType(f, _) => s"Applied(${chain(f, depth + 1)})"
        case _ => kind(t)
    def owners(s: Symbol): String =
      if s.isNoSymbol then "NoSymbol" else if s == defn.RootClass then "<root>" else s"${s.name}/${s.isPackageDef} > ${owners(s.owner)}"
    val t = TypeRepr.of[T]
    val base = t match
      case AppliedType(f, _) => f
      case other => other
    val q = base match
      case r: NamedType => r.qualifier
      case other => other
    val lines = List(
      chain(t, 0),
      s"q: ${kind(q)} typeSym=${q.typeSymbol.name} pkg=${q.typeSymbol.isPackageDef} exists=${q.typeSymbol.exists} term=${q.termSymbol.isNoSymbol} full=${q.typeSymbol.fullName}",
      s"owners: ${owners(base.typeSymbol)}",
      s"full: ${base.typeSymbol.fullName} q-module-full=${q.typeSymbol.companionModule.fullName}",
      s"pkg-termref: ${chain(q.typeSymbol.companionModule.termRef, 0)} typeref: ${chain(q.typeSymbol.typeRef, 0)}"
    )
    Expr(lines.mkString("\n"))
