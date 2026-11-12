package rounds

import scala.quoted.*

object Fixing:
  inline def level: Int = ${ levelImpl }
  def levelImpl(using Quotes): Expr[Int] = Expr(42)

@main def run(): Unit =
  println(s"${Reads.early} ${Reads.fixed} ${Later.again}")
