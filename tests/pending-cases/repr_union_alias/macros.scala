// A union alias's reference is its TypeRef (dotty's `TypeRef` to a `TypeAlias`): scalac's reflection sees `O.U` and
// its symbol, teq's the union and no symbol. Moves with type-representation's landing 4.
import scala.quoted.*
object Reflect:
  inline def shape[T]: String = ${ shapeImpl[T] }
  def shapeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | " + t.show(using Printer.TypeReprStructure) + " | " + t.typeSymbol.fullName)
