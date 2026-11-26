// A macro that compares the owners of two terms by symbol identity.
import scala.quoted.*
object Inspect {
 inline def same[A, B]: Boolean = ${ impl[A, B] }
 def impl[A: Type, B: Type](using q: Quotes): Expr[Boolean] = {
  import q.reflect.*
  Expr(TypeRepr.of[A].termSymbol.owner == TypeRepr.of[B].termSymbol.owner)
 }
}
