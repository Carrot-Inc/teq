package blib
import scala.deriving.Mirror
import scala.quoted.*

object Shape:
  inline def of[B <: Product](using m: Mirror.Of[B]): String = ${ ofImpl[B]('m) }

  def isProduct[P <: Product: Type](using Quotes): Boolean =
    import quotes.reflect.*
    TypeRepr.of[P] <:< TypeRepr.of[Product]

  def ofImpl[B <: Product: Type](m: Expr[Mirror.Of[B]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    m match
      case '{ $p: Mirror.ProductOf[B] { type MirroredElemTypes = EmptyTuple } } => Expr("none")
      case '{ $p: Mirror.ProductOf[B] { type MirroredElemTypes = t *: EmptyTuple } } => Expr("one " + Type.show[t])
      case '{ type t <: Product; $p: (Mirror.ProductOf[B] { type MirroredElemTypes = t }) } =>
        Expr("many " + isProduct[t])
