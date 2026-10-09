import scala.quoted.*

// A top-level object is a member of its package, not of the file's package object: the owner
// and full name izumi-reflect reads for the prefix of a class nested in one. `NoSymbol.termRef`
// is a reference to nothing.
object Macros:
  inline def prefixOf[T]: String = ${ prefixOfImpl[T] }
  def prefixOfImpl[T: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    TypeRepr.of[T] match
      case tr: TypeRef =>
        val module = tr.qualifier.typeSymbol.companionModule
        Expr(s"${module.fullName} ${module.owner.fullName} ${module.owner.isPackageDef} ${Symbol.noSymbol.termRef.termSymbol.isNoSymbol}")
      case _ => Expr("no type reference")
