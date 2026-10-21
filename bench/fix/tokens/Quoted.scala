import scala.quoted.*
object Q:
  val x = 1
  def q(using Quotes): Expr[Any] = '{ (${ 'x }, ${ 'null }, ${ 'this }, ${ 'true }) }
