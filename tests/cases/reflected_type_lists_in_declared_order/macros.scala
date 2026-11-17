// What `Symbol.declaredTypes`, `Symbol.typeMembers` and `Symbol.declarations` list for a class, in
// the order scalac gives each: `declaredTypes` the type parameters, then the nested classes and the
// type members as the class declares them; `typeMembers` the type parameters, then the members a
// lookup finds, the class's own first and each declared parent's after in the parents' order, one
// owner's by name (`QuotesImpl.typeMembers`); `declarations` its nested classes in the declared
// order.
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
