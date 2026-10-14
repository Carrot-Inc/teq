// An extension of a given in scope selected with type arguments for the method's own type parameters
// (`pf.asExprOf[Any]`, `Quotes`' `extension (self: Expr[Any]) def asExprOf[X](using Type[X])`): the `[X]` fixes the
// `using Type[X]` the application resolves, which the ranking of the givens providing the extension leaves to it
// (dotty's `tryExtension` under a `PolyProto`), so that two `Type` evidences in scope (`A: Type, B: Type`) do not
// make it ambiguous and pass the extension over (Airstream's split macros).
import scala.quoted.*

object Macros:
  inline def count[A, B](inline pf: PartialFunction[A, B], inline more: Any*): Int = ${ countImpl('pf, 'more) }

  def countImpl[A: Type, B: Type](pf: Expr[PartialFunction[A, B]], more: Expr[Seq[Any]])(using quotes: Quotes): Expr[Int] =
    val rest = more match
      case Varargs(es) => es
      case _ => Nil
    '{ List(${ Varargs(rest :+ pf.asExprOf[Any]) }*).size }

  inline def shown[A, B](inline m: Map[A, B]): String = ${ shownImpl('m) }

  def shownImpl[A: Type, B: Type](m: Expr[Map[A, B]])(using Quotes): Expr[String] =
    val any: Expr[Any] = m.asExprOf[Any]
    '{ ${ any }.toString }
