package quotedcopies
import scala.quoted.*
object Macros:
  inline def number: Int = 3
  inline def twice(inline x: Int): Int = ${ twiceImpl('x) }
  def twiceImpl(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x * 2 }
  inline def nested: Int = ${ nestedImpl }
  def nestedImpl(using Quotes): Expr[Int] = '{ twice(number) + 1 }
