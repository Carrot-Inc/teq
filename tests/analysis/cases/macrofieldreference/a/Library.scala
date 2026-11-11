package macrofieldreference
import scala.quoted.*
trait Read[A]:
  extension (a: A) def value: Long
object Read:
  transparent inline def derived[A]: Read[A] = ${ fieldImpl[A] }
  def fieldImpl[A: Type](using Quotes): Expr[Read[A]] =
    import quotes.reflect.*
    val owner = TypeRepr.of[A].typeSymbol
    val field = owner.caseFields.head
    def select(value: Expr[A]): Expr[Long] = Select(value.asTerm, field).asExprOf[Long]
    '{ new Read[A]:
      extension (a: A) def value: Long = ${ select('a) }
    }
