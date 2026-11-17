import scala.quoted.*

object Lists {
  inline def members[A]: String = ${ impl[A] }

  private def impl[A: Type](using Quotes): Expr[String] = {
    import quotes.reflect.*
    val s = TypeRepr.of[A].typeSymbol
    Expr(s.name + " typeMembers: " + s.typeMembers.map(_.name).filterNot(_.endsWith("$")).mkString(", "))
  }
}
