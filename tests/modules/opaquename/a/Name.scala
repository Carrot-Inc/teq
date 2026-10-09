package ona

import scala.quoted.*

// The full name of a type as a macro reads it: an upstream's opaque type is a member of its
// file's `Id$package` object, as scalac's reflection has it.
object Name:
  inline def of[T]: String = ${ ofImpl[T] }
  def ofImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    Expr(TypeRepr.of[T].typeSymbol.fullName)
