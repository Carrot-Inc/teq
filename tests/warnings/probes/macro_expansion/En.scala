package core
import scala.quoted.*
trait En[A] { def n: Int }
object En:
  transparent inline def derived[A]: En[A] = ${ impl[A] }
  def impl[A: Type](using Quotes): Expr[En[A]] =
    import quotes.reflect.*
    val comp = Ref(TypeRepr.of[A].typeSymbol.companionModule).asExprOf[Any]
    '{ new En[A] { def n = $comp.hashCode } }
