import scala.quoted.*
trait Box:
  def v: Int
object X:
  inline given box: Box = ${ impl }
  def impl(using Quotes): Expr[Box] = '{ new Box { def v = 42 } }
