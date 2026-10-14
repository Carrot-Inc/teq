// An annotation whose class an export forwards: `@tag` names the export's type forwarder, an alias
// of the class, whose instance a macro reads from `Symbol.annotations` (dotty's `New` of an alias).
package p.json

import scala.quoted.*

final class tag(val name: String) extends scala.annotation.StaticAnnotation

object Reflect:
  inline def tagOf[A]: Option[String] = ${ tagOfImpl[A] }
  private def tagOfImpl[A: Type](using Quotes): Expr[Option[String]] =
    import quotes.reflect.*
    val found = TypeRepr.of[A].typeSymbol.annotations.collectFirst {
      case Apply(Select(New(tpt), _), List(Literal(StringConstant(name)))) if tpt.tpe.typeSymbol.name == "tag" => name
    }
    Expr(found)
