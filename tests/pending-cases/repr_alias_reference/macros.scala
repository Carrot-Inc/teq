// An alias's reference is a TypeRef kept until a reader expands it (dotty's `TypeRef`, `dealias`, Types.scala
// 1546-1572): scalac's reflection sees `O.P`, teq's the expansion `Int`. Moves with type-representation's landing 4.
import scala.quoted.*
object Reflect:
  inline def shape[T]: String = ${ shapeImpl[T] }
  def shapeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | " + t.show(using Printer.TypeReprStructure) + " | " + t.typeSymbol.fullName)
