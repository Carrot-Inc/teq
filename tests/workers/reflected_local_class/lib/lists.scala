import scala.quoted.*

object Lists {
  inline def of[A]: List[String] = ${ impl[A] }

  private def impl[A: Type](using Quotes): Expr[List[String]] = {
    import quotes.reflect.*
    val s = TypeRepr.of[A].typeSymbol
    def names(xs: List[Symbol]): String = xs.map(_.name).filterNot(_.endsWith("$")).mkString(", ")
    Expr(List(
      s.name + " declaredTypes: " + names(s.declaredTypes),
      s.name + " typeMembers: " + names(s.typeMembers),
      s.name + " declarations (classes): " + names(s.declarations.filter(_.isClassDef)),
    ))
  }
}
