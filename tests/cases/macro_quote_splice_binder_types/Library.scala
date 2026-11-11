import scala.quoted.*
object Producer:
  transparent inline def of[A]: Factory[A] = new Factory[A]
class Factory[A]:
  inline def duplicate: A => A = ${ Inspect.duplicate[A] }
object Inspect:
  def duplicate[A: Type](using Quotes): Expr[A => A] =
    import quotes.reflect.*
    '{ (item: A) => ${
      val receiver = 'item.asTerm
      Select.unique(receiver, "copy").appliedToArgs(List(Select.unique(receiver, "value"))).asExprOf[A]
    } }
trait Projection[A]:
  def read(item: A): Int
inline def projection[A]: Projection[A] = ${ InspectProjection.impl[A] }
object InspectProjection:
  def impl[A: Type](using Quotes): Expr[Projection[A]] =
    import quotes.reflect.*
    val field = TypeRepr.of[A].typeSymbol.caseFields.head
    '{ new Projection[A] {
      def read(item: A): Int = ${ Select('item.asTerm, field).asExprOf[Int] }
    } }
