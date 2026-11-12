// `dealias` keeps an abstract type's reference (Types.scala 1546-1572: only a `TypeAlias` expands):
// scalac's reflection gives `A.T`, and teq's is to give it too. Moves with type-representation.
import scala.quoted.*
object MoreReflect:
  inline def info[T]: String = ${ impl[T] }
  def impl[T: Type](using Quotes): Expr[String] =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    Expr(t.show + " | dealias=" + t.dealias.show + " | widen=" + t.widen.show)
