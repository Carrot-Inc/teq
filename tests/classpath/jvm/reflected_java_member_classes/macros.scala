// jars: scala-library nesting-lib
// A Java class's member classes as a macro lists them: `declaredTypes` the member classes in the
// order of the class file's `InnerClasses` entries, with the type parameters after them
// (`ClassfileParser` enters the inner classes before it reads the signature), which is scalac's
// order for a class file of seven entries or fewer, as here; `typeMembers` the type parameters
// first, then the classes by name; a Scala child's `typeMembers` holds the parent's member classes
// but its private one.
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
