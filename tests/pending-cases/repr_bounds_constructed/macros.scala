// Bounds a macro makes are real bounds, equal ends kept (`TypeBounds.apply`, Types.scala 5844-5845): scalac's
// reflection reads `Int`/`Int` back, teq's `Nothing`/`Any`. Moves with type-representation's landing 2.
import scala.quoted.*
object Reflect:
  inline def bounds: String = ${ boundsImpl }
  def boundsImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val b = TypeBounds(TypeRepr.of[Int], TypeRepr.of[Int])
    Expr(b.show(using Printer.TypeReprStructure) + " | " + b.low.show + " | " + b.hi.show)
