package mlib
import scala.quoted.*

object Params:
  inline def of[A]: String = ${ ofImpl[A] }

  def ofImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    def param(t: TypeRepr): String = t match
      case ByNameType(u) => "=> " + u.typeSymbol.name
      case AppliedType(tc, List(e)) => tc.typeSymbol.name + "[" + e.typeSymbol.name + "]"
      case other => other.typeSymbol.name
    def info(t: TypeRepr): String = t match
      case m: MethodType => m.paramTypes.map(param).mkString("(", ", ", ")") + info(m.resType)
      case other => ": " + other.typeSymbol.name
    TypeRepr.of[A] match
      case r: Refinement =>
        val i = r.info
        val rebuilt = Refinement(r.parent, r.name, i)
        Expr(r.name + info(i) + " " + info(rebuilt.info))
      case _ => Expr("none")
