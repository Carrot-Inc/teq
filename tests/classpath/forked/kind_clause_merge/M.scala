import scala.quoted.*
object M:
  inline def one: Int = ${ oneImpl }
  def oneImpl(using Quotes): Expr[Int] = '{ 1 }
