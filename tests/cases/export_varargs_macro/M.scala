package mm
import scala.quoted.*
object Impl:
  def countImpl(xs: Expr[Seq[Int]])(using Quotes): Expr[Int] = xs match
    case Varargs(es) => Expr(es.size)
    case _ => quotes.reflect.report.errorAndAbort("arguments have to be written out")
inline def count(inline xs: Int*): Int = ${ Impl.countImpl('xs) }
