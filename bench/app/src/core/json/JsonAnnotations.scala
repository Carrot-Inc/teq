package meridian.core.json

import scala.quoted.*

/** Reads the annotations a derivation needs from the type at compile time. */
object JsonAnnotations:
  inline def discriminatorOf[A]: Option[String] = ${ discriminatorImpl[A] }

  /** The values of an enum whose cases are all singletons, for the sums too wide to walk. */
  inline def enumValuesOf[A]: List[A] = ${ enumValuesImpl[A] }

  private def enumValuesImpl[A: Type](using Quotes): Expr[List[A]] =
    import quotes.reflect.*
    val symbol = TypeRepr.of[A].typeSymbol
    if symbol.flags.is(Flags.Enum) && symbol.companionModule.exists then
      '{ ${ Select.unique(Ref(symbol.companionModule), "values").asExprOf[Array[A]] }.toList }
    else '{ Nil }

  private def discriminatorImpl[A: Type](using Quotes): Expr[Option[String]] =
    import quotes.reflect.*
    val symbol = TypeRepr.of[A].typeSymbol
    val found = symbol.annotations.collectFirst {
      case Apply(Select(New(tpt), _), List(Literal(StringConstant(name)))) if tpt.tpe.typeSymbol.name == "jsonDiscriminator" => name
    }
    found match
      case Some(name) => '{ Some(${ Expr(name) }) }
      case None => '{ None }
