// An alias's reference is a TypeRef kept until a reader expands it (dotty's `TypeRef`, `dealias`, Types.scala
// 1546-1572): scalac's reflection sees `O.P`, and teq's is to see it too. Moves with type-representation.
import scala.quoted.*
object Reflect:
  inline def shape[T]: String = ${ shapeImpl[T] }
  def shapeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | " + t.show(using Printer.TypeReprStructure) + " | " + t.typeSymbol.fullName)
