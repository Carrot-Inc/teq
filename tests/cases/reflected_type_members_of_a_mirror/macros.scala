// The type members a macro finds in `Mirror` and in the classes that extend it: `Mirror` itself
// declares `MirroredMonoType`, `MirroredLabel` and `MirroredElemLabels`; `MirroredType` and
// `MirroredElemTypes` are members of `Mirror.Singleton` and `Mirror.SingletonProxy`, which define
// them, and of no other mirror class: a mirror's type has them by its refinement. So a case
// class's companion, a `Mirror.Product`, has neither, and a case object, a `Mirror.Singleton`,
// has both.
import scala.quoted.*

object Members {
  inline def has[A](inline name: String): Boolean = ${ hasImpl[A]('name) }
  inline def typeNames[A]: String = ${ namesImpl[A] }

  private def hasImpl[A: Type](name: Expr[String])(using Quotes): Expr[Boolean] = {
    import quotes.reflect.*
    val n = name.valueOrAbort
    Expr(TypeRepr.of[A].typeSymbol.typeMembers.exists(_.name == n))
  }

  private def namesImpl[A: Type](using Quotes): Expr[String] = {
    import quotes.reflect.*
    val s = TypeRepr.of[A].typeSymbol
    def names(xs: List[Symbol]): String = xs.filterNot(_.isTypeParam).map(_.name).mkString(", ")
    Expr(s.name + " declares " + names(s.declaredTypes) + " | has " + names(s.typeMembers))
  }
}
