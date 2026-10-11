import scala.quoted.*

// Case classes of the program whose selectors the macro calls as it expands, one through the
// abstract member of a trait the selector implements.
trait First:
  def _1: Int

case class P(a: Int, b: String)
case class Q(n: Int) extends First

object M:
  inline def first: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    val p = P(41, "x")
    val f: First = Q(7)
    quotes.reflect.report.error(s"read ${p._1 + 1}, ${p._2} and ${f._1}")
    Expr(p._1)
