import scala.quoted.*

// A macro's run gives an info message and then counts in its object, which other runs read:
// the attempt gives way to one worker, and the messages are printed once, the serial build's.
object Counted:
  var n = 0
  def next(e: Expr[String])(using Quotes): Expr[Int] =
    quotes.reflect.report.info(s"counting ${e.valueOrAbort}")
    n += 1
    Expr(n)

inline def next(inline s: String): Int = ${ Counted.next('s) }
