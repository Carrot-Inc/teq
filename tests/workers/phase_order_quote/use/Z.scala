import scala.quoted.*

object Z:
  val z = M.next
  def unused(using Quotes): Expr[Int] = '{ 0 }
