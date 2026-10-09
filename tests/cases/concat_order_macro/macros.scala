//> using platform js
import scala.quoted.*

object Macros:
  inline def joined(inline a: Any, inline b: String): String = ${ joinedImpl('a, 'b) }
  def joinedImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ "q:" + $a + $b + ${ a }.toString }

  inline def folded(inline xs: Any*): String = ${ foldedImpl('xs) }
  def foldedImpl(xs: Expr[Seq[Any]])(using Quotes): Expr[String] =
    xs match
      case Varargs(elems) => elems.foldLeft('{ "" })((acc, e) => '{ $acc + $e })
