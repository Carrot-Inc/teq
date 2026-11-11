// A quote run twice: the first run's tree, typed again at its splice, selects the branch of a constant
// condition; the second run's tree, inspected by reflection, is still the quote's `if`.
import scala.quoted.*

object Conditions:
  def cond(): true = { println("condition"); true }

object M:
  def tree(using Quotes): Expr[String] =
    '{ if Conditions.cond() then "a" else "b" }

  inline def expand: String = ${ tree }
  inline def shape: Boolean = ${ shapeImpl }

  def shapeImpl(using q: Quotes): Expr[Boolean] =
    import q.reflect.*
    def isIf(t: Term): Boolean = t match
      case Inlined(_, _, body) => isIf(body)
      case If(_, _, _) => true
      case _ => false
    Expr(isIf(tree.asTerm))
