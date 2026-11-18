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
// A quote's braces with a template opened on their own line: the braces take the width of their first line break's
// line, the template's body line, so that line is no wider than its region and opens no indented body. scalac:
// "indented definitions expected, identifier extension found" at the body's first token.
// expect: 13:7: error:
