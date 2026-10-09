import scala.deriving.Mirror
import scala.quoted.*

trait Named[A]:
  def name: String

// A plain inline given whose macro reads its inline mirror's refinement, as circe's derivation
// does: the search answers with the call, expanded after the body is typed, and the mirror the
// search synthesized keeps its own type for the expansion.
object Named:
  inline given derived[A](using inline m: Mirror.Of[A]): Named[A] = ${ derivedImpl[A]('m) }
  def derivedImpl[A: Type](m: Expr[Mirror.Of[A]])(using Quotes): Expr[Named[A]] =
    m match
      case '{ $p: Mirror.ProductOf[A] { type MirroredLabel = l } } =>
        val label = Type.valueOfConstant[l].fold("?")(_.toString)
        '{ new Named[A] { def name: String = ${ Expr("product " + label) } } }
      case '{ $s: Mirror.SumOf[A] { type MirroredLabel = l } } =>
        val label = Type.valueOfConstant[l].fold("?")(_.toString)
        '{ new Named[A] { def name: String = ${ Expr("sum " + label) } } }
      case _ =>
        quotes.reflect.report.errorAndAbort("no mirror of a known shape")
