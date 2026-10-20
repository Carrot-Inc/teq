import scala.quoted.*

// A macro's run before the fork (its file names quoted code) changes a counter the runs on the
// workers read.
object State:
  var n: Int = 0
  def incImpl(using Quotes): Expr[Int] =
    n += 1
    Expr(n)
  def readImpl(using Quotes): Expr[Int] =
    Expr(n)

transparent inline def inc: Int = ${ State.incImpl }
transparent inline def mh: Int = ${ State.readImpl }
