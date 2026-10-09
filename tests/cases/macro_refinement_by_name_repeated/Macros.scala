package mlib
import scala.quoted.*

object Infos:
  inline def compare[A, B]: String = ${ compareImpl[A, B] }

  def compareImpl[A: Type, B: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    val (ra, rb) = (TypeRepr.of[A].asInstanceOf[Refinement], TypeRepr.of[B].asInstanceOf[Refinement])
    val rebuilt = Refinement(ra.parent, ra.name, ra.info)
    def param(t: TypeRepr): String = t match
      case ByNameType(u) => "=> " + param(u)
      case AppliedType(tc, List(e)) => tc.typeSymbol.name + "[" + e.typeSymbol.name + "]"
      case other => other.typeSymbol.name
    val params = ra.info match
      case m: MethodType => m.paramTypes.map(param).mkString(", ")
      case _ => "-"
    Expr(s"${ra.info =:= rb.info} ${rebuilt =:= TypeRepr.of[A]} $params")
