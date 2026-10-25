// A by-name type is an `ExprType`, its symbol its result's (Types.scala 3812-3844, `typeSymbol` 617-622): scalac's
// reflection gives `scala.Int`, teq's its synthetic class `scala.<byname>`. Moves with type-representation's landing 3.
import scala.quoted.*
object Reflect:
  inline def byname: String = ${ bynameImpl }
  def bynameImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = ByNameType(TypeRepr.of[Int])
    Expr(t.widen.show(using Printer.TypeReprStructure) + " | " + t.widenByName.show + " | symbol=" + t.typeSymbol.fullName)
