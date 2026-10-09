import scala.quoted.*
class B(val v: Int)
object O extends B(1)
object M:
  def showImpl[U: Type](x: Expr[U])(using Quotes): Expr[String] = Expr(Type.show[U])
  inline def show[T](x: T): String = ${ showImpl('x) }
  inline def showB(x: B): String = ${ showImpl('x) }
  inline def showInline(inline x: B): String = ${ showImpl('x) }
  inline def showList[T](xs: List[T]): String = ${ showImpl('xs) }
  inline def pair[T](x: T): String = ${ showImpl('{ (x, x) }) }
  def pick(x: Expr[Any])(using Quotes): Expr[String] = Expr("any")
  def pick(x: Expr[Int])(using Quotes, Type[Int]): Expr[String] = Expr("int")
  inline def which[T](x: T): String = ${ pick('x) }
  inline def whichInt(x: Int): String = ${ pick('x) }
