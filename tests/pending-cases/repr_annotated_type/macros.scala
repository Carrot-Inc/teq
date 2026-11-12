// An annotated type is `AnnotatedType(parent, annot)`, the annotation's arguments kept (Types.scala 5883
// on): scalac's reflection sees `Int @mark(7)`, and teq's is to see it too. Moves with type-representation.
import scala.quoted.*
object Reflect:
  inline def shape[T]: String = ${ shapeImpl[T] }
  def shapeImpl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | " + t.show(using Printer.TypeReprStructure) + " | " + t.typeSymbol.fullName)
