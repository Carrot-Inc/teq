// jars: scala-library nesting-lib
// A Java class whose `InnerClasses` attribute has eight entries or more: `declaredTypes` and
// `declarations` give its member classes in the class file's order (here Rod, Tray, Bin, Hook,
// Peg, Rail, Lock, Box, Tin), then its type parameters. scalac gives them in the order of its own
// table of the attribute's entries, which is a hash table from the eighth entry on
// (`ClassfileParser.scala` 1067 walks `innerClasses.valuesIterator`; `GenericHashMap`'s dense
// limit is 8): for this class Tin, Rod, Hook, Box, Bin, Rail, Lock, Peg, Tray. The expectation
// holds the class file's order and is written by hand; `typeMembers`, ordered by name, is
// scalac's.
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
