import scala.quoted.*

object Macros:
  inline def answer: Int = ${ answerImpl }
  def answerImpl(using Quotes): Expr[Int] = Expr(42)
