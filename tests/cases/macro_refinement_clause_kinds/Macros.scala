package mlib
import scala.quoted.*

object Infos:
  inline def compare[A, B]: String = ${ compareImpl[A, B] }

  def compareImpl[A: Type, B: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    val (ra, rb) = (TypeRepr.of[A].asInstanceOf[Refinement], TypeRepr.of[B].asInstanceOf[Refinement])
    val rebuilt = Refinement(ra.parent, ra.name, ra.info)
    val kinds = ra.info match
      case m: MethodType => s"${m.isImplicit} ${m.isContextual}"
      case _ => "-"
    Expr(s"${ra.info =:= rb.info} ${rebuilt =:= TypeRepr.of[A]} $kinds")
