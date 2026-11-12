package rounds

import scala.quoted.*

object Fixing:
  inline def level: Int = ${ levelImpl }
  def levelImpl(using Quotes): Expr[Int] = Expr(42)
  val again: Int = Counting.sized

@main def run(): Unit =
  println(s"${Reads.counted} ${Reads.fixed} ${Fixing.again} ${rounddemand.Defaults.sized(Array(8, 2, 6, 5))}")
