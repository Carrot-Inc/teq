package mlib
import scala.quoted.*

object Members:
  inline def of[A]: String = ${ ofImpl[A] }

  def ofImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    def name(t: TypeRepr): String = t match
      case AppliedType(tc, args) => tc.typeSymbol.name + args.map(name).mkString("[", ", ", "]")
      case _ => t.typeSymbol.name
    def info(t: TypeRepr): String = t match
      case ByNameType(u) => "=> " + name(u)
      case m: MethodType => m.paramNames.zip(m.paramTypes.map(name)).map(_ + ": " + _).mkString("(", ", ", ")") + info(m.resType)
      case TypeBounds(lo, hi) if lo =:= hi => "= " + name(hi)
      case TypeBounds(lo, hi) => ">: " + name(lo) + " <: " + name(hi)
      case other => ": " + name(other)
    def flatten(t: TypeRepr): (TypeRepr, List[(String, TypeRepr)]) = t match
      case r: Refinement =>
        val (p, ms) = flatten(r.parent)
        (p, ms :+ (r.name, r.info))
      case other => (other, Nil)
    val (parent, members) = flatten(TypeRepr.of[A])
    val read = members.map((n, i) => n + " " + info(i))
    val rebuilt = members.foldLeft(TypeRepr.of[AnyRef]) { case (p, (n, i)) => Refinement(p, n, i) }
    def unapplied(t: TypeRepr): List[String] = t match
      case Refinement(p, n, i) => unapplied(p) :+ (n + " " + info(i))
      case _ => Nil
    Expr((name(parent) :: read).mkString("; ") + " | " + unapplied(rebuilt).mkString("; "))
