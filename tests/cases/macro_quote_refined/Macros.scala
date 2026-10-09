package mlib
import scala.deriving.Mirror
import scala.quoted.*

object Labels:
  inline def of[A]: String = ${ ofImpl[A] }

  private def unroll(using q: Quotes)(t: q.reflect.TypeRepr): List[q.reflect.TypeRepr] =
    import q.reflect.*
    t.dealias match
      case AppliedType(tc, List(h, tl)) if tc.typeSymbol.name == "*:" => h :: unroll(tl)
      case AppliedType(tc, args) if tc.typeSymbol.name.startsWith("Tuple") => args
      case _ => Nil

  private def label(using q: Quotes)(t: q.reflect.TypeRepr): String =
    import q.reflect.*
    t match
      case ConstantType(c) => c.value.toString
      case other => other.typeSymbol.name

  def ofImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    Expr.summon[Mirror.ProductOf[A]] match
      case Some('{ $m: Mirror.Product { type MirroredElemLabels = labels; type MirroredElemTypes = types } }) =>
        val names = unroll(TypeRepr.of[labels]).map(label)
        val tpes = unroll(TypeRepr.of[types]).map(label)
        Expr(names.zip(tpes).map((n, t) => n + ": " + t).mkString(", "))
      case _ => Expr("none")
