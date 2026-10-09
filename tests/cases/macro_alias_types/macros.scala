// A macro sees a declared type as written: an alias application stays `Lens[S, A]` in `show`,
// `AppliedType` and `typeSymbol`, and `dealias` reaches `PLens[S, S, A, A]`, as scalac's
// reflect API gives them.
import scala.quoted.*

package optics:
  class PLens[S, T, A, B](val get: S => A, val set: B => S => T):
    def andThen[C, D](other: PLens[A, B, C, D]): PLens[S, T, C, D] =
      PLens(s => other.get(get(s)), d => s => set(other.set(d)(get(s)))(s))
  type Lens[S, A] = PLens[S, S, A, A]
  object Lens:
    def apply[S, A](get: S => A)(set: A => S => S): Lens[S, A] = new PLens(get, set)
  object Holder:
    type Optional[S, A] = PLens[S, S, Option[A], Option[A]]
    def opt[S, A](get: S => Option[A]): Optional[S, A] = new PLens(get, o => s => s)

object Macros:
  inline def describe[T]: String = ${ describeImpl[T] }
  def describeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(line(TypeRepr.of[T]))

  inline def describeTerm(inline x: Any): String = ${ describeTermImpl('x) }
  def describeTermImpl(x: Expr[Any])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = x.asTerm.tpe
    Expr(line(t) + " | widened " + line(t.widen))

  inline def describeMember[T](inline name: String): String = ${ describeMemberImpl[T]('name) }
  def describeMemberImpl[T: Type](name: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val cls = TypeRepr.of[T].dealias.typeSymbol
    val member = cls.methodMember(name.valueOrAbort).head
    val declared = member.tree match
      case DefDef(_, paramss, tpt, _) =>
        val params = paramss.flatMap(_.params).collect { case ValDef(n, pt, _) => n + ": " + pt.tpe.show }
        "result " + line(tpt.tpe) + "; params " + params.mkString(", ")
      case ValDef(_, tpt, _) => "val " + line(tpt.tpe)
      case other => "other " + other.show
    Expr(declared)

  inline def relate[A, B]: String = ${ relateImpl[A, B] }
  def relateImpl[A: Type, B: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val (a, b) = (TypeRepr.of[A], TypeRepr.of[B])
    Expr(s"${a.show} <:< ${b.show}: ${a <:< b}, =:= ${a =:= b}, same symbol ${a.typeSymbol == b.typeSymbol}, same after dealias ${a.dealias.typeSymbol == b.dealias.typeSymbol}")

  private def line(using Quotes)(t: quotes.reflect.TypeRepr): String =
    import quotes.reflect.*
    val shape = t match
      case AppliedType(tycon, args) => s"AppliedType(${tycon.show}, ${args.length} args, last ${args.last.show})"
      case _ => "not applied"
    s"${t.show} / dealias ${t.dealias.show} / symbol ${t.typeSymbol.name} / dealias symbol ${t.dealias.typeSymbol.name} / $shape / alias ${t.typeSymbol.isAliasType}"
