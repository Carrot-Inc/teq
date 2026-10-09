package mlib
import scala.quoted.*

object Infos:
  inline def same[A]: String = ${ sameImpl[A] }

  def sameImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    TypeRepr.of[A] match
      case r: Refinement =>
        val i = r.info
        val rebuilt = Refinement(r.parent, r.name, i)
        Expr(s"${i =:= r.info} ${i =:= rebuilt.info} ${rebuilt =:= TypeRepr.of[A]}")
      case _ => Expr("none")
