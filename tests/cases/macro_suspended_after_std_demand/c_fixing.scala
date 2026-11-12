package rounds

import scala.quoted.*

object Fixing:
  inline def level: Int = ${ levelImpl }
  def levelImpl(using Quotes): Expr[Int] = Expr(42)
  val again: Int = Counting.pair

@main def run(): Unit =
  println(s"${Reads.counted} ${Reads.fixed} ${Fixing.again}")
