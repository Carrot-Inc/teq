package recmu

import scala.quoted.*

// The macro reads the upstream's types as the pickles give them: a recursive type, or not.
object Inspect:
  inline def recursive[A]: String = ${ recursiveImpl[A] }
  def recursiveImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    TypeRepr.of[A].dealias match
      case rt: RecursiveType => Expr("rec " + show(rt.underlying))
      case other => report.errorAndAbort("not a recursive type: " + other.show)

  inline def plain[A]: String = ${ plainImpl[A] }
  def plainImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    TypeRepr.of[A].dealias match
      case rt: RecursiveType => report.errorAndAbort("a recursive type: " + rt.show)
      case other => Expr("plain " + show(other))

  def show(using q: Quotes)(t: q.reflect.TypeRepr): String =
    import q.reflect.*
    t match
      case r: Refinement => show(r.parent) + " { " + r.name + " " + info(r.info) + " }"
      case TypeRef(RecursiveThis(_), name) => "self." + name
      case TypeRef(_, name) => name
      case _ => "?"
  def info(using q: Quotes)(t: q.reflect.TypeRepr): String =
    import q.reflect.*
    t match
      case TypeBounds(lo, hi) if lo =:= hi => "= " + show(hi)
      case TypeBounds(lo, hi) => ">: " + show(lo) + " <: " + show(hi)
      case m: MethodType => m.paramNames.mkString("(", ", ", ")") + ": " + show(m.resType)
      case other => ": " + show(other)
