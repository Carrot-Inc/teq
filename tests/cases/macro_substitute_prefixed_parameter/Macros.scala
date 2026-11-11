// Substituters replaces a NamedType's own symbol before its prefix.
import scala.quoted.*

class Box[A](val value: A)

object DesignatorProbe:
  inline def replaced: Boolean = ${ replacedImpl }

  def replacedImpl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    val box = TypeRepr.of[Box[?]]
    val field = box.typeSymbol.fieldMember("value")
    val raw = box.memberType(field)
    val result = raw.substituteTypes(List(raw.typeSymbol), List(TypeRepr.of[String]))
    Expr(result =:= TypeRepr.of[String])
