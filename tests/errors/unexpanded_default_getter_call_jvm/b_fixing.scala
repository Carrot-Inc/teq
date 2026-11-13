package rounds

import scala.quoted.*

object Fixing:
  inline def answer: Int = ${ answerImpl }
  def answerImpl(using Quotes): Expr[Int] = Expr(42)
