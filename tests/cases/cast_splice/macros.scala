// A cast a macro's quote writes is decided where the quote runs, its hole filled and its type
// arguments given: it checks the class, or unboxes.
import scala.quoted.*

class A
class B

object M:
  inline def cast(inline x: Any): B = ${ castImpl('x) }
  def castImpl(x: Expr[Any])(using Quotes): Expr[B] = '{ $x.asInstanceOf[B] }
  inline def castTo[T](inline x: Any): T = ${ castToImpl[T]('x) }
  def castToImpl[T: Type](x: Expr[Any])(using Quotes): Expr[T] = '{ $x.asInstanceOf[T] }
