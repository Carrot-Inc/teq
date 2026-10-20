import scala.quoted.*
object X:
  val a: Int = P.next
  def impl(using Quotes): Expr[Int] = Expr(100)
  inline def hundred: Int = ${ impl }
