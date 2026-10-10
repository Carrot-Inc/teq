import scala.quoted.*

// Counters of their expansions on one count: a transparent number and a transparent array of one.
object M:
  var n = 0
  transparent inline def next: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
  transparent inline def arr: Array[Int] = ${ arrImpl }
  def arrImpl(using Quotes): Expr[Array[Int]] =
    n += 1
    val e = Expr(n)
    '{ Array($e) }
