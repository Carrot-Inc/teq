// Bounds a macro makes are real bounds, equal ends kept (`TypeBounds.apply`, Types.scala 5844-5845): scalac's
// reflection reads `Int`/`Int` back, and teq's is to read them back too. Moves with type-representation.
import scala.quoted.*
object Reflect:
  inline def bounds: String = ${ boundsImpl }
  def boundsImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val b = TypeBounds(TypeRepr.of[Int], TypeRepr.of[Int])
    Expr(b.show(using Printer.TypeReprStructure) + " | " + b.low.show + " | " + b.hi.show)
