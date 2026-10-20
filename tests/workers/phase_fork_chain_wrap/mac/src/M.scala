package sample
import scala.quoted.*
trait R { def n: Int }
object M:
  inline def make: R = ${ impl }
  transparent inline def wrap: R = make
  def impl(using Quotes): Expr[R] = '{ class Made extends R { def n: Int = 7 }; new Made }
