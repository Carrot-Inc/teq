// `Symbol.typeMembers` holds what a member lookup finds on the class (`Type.typeMembers`): a
// parent's private type and private class are no members of the child, its protected and
// qualified-private ones are, and the class's own private ones stay; a parent named through an
// alias counts as its class. One owner's members are ordered as `String.compareTo` orders their
// names, by UTF-16 code units: a name outside the basic plane sorts before U+E000.
import scala.quoted.*

object Lists {
  inline def of[A]: List[String] = ${ impl[A] }
  inline def units[A]: String = ${ unitsImpl[A] }

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

  // The type members' names, each as its UTF-16 code units.
  private def unitsImpl[A: Type](using Quotes): Expr[String] = {
    import quotes.reflect.*
    def codeUnits(name: String, from: Int): String =
      if (from >= name.length) ""
      else (if (from > 0) "-" else "") + name.charAt(from).toInt + codeUnits(name, from + 1)
    Expr(TypeRepr.of[A].typeSymbol.typeMembers.map(s => codeUnits(s.name, 0)).mkString(", "))
  }
}
