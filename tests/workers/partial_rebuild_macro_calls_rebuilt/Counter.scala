package inc
import scala.quoted.*
object Counter:
  transparent inline def sized: Int = ${ sizedImpl }
  def sizedImpl(using Quotes): Expr[Int] = Expr(Stock.compute(2))
