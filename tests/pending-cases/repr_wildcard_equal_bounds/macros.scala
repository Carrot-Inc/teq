// A wildcard argument is real bounds, equal ends kept (`TypeBounds.apply`, Types.scala 5844-5845): scalac's
// reflection sees `List[_ >: Int <: Int]`, and teq's is to see it too. Moves with type-representation.
import scala.quoted.*
object Reflect:
  inline def shape[T]: String = ${ shapeImpl[T] }
  def shapeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | " + t.show(using Printer.TypeReprStructure) + " | " + t.typeSymbol.fullName)
