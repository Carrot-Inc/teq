// `typedInfixOp` lifts nothing in a quote pattern (`Mode.QuotedExprPattern`): the pattern `$h :: $t` is the
// application itself and matches a cons whose operand is pure; a cons whose operand is impure was lifted where it
// was written and is a block, which the pattern does not match.
import scala.quoted.*
object M:
  inline def shape(inline e: List[Int]): String = ${ shapeImpl('e) }
  def shapeImpl(e: Expr[List[Int]])(using Quotes): Expr[String] =
    e match
      case '{ ($h: Int) :: ($t: List[Int]) } => Expr("cons")
      case _ => Expr("other")
