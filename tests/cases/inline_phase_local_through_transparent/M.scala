// A counter macro: each expansion takes the next number.
import scala.quoted.*

object M:
  private var count = 0
  inline def next: Int = ${ nextImpl }
  def nextImpl(using Quotes): Expr[Int] =
    count += 1
    Expr(count)
