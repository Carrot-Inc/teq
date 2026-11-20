import scala.quoted.*

// A macro whose run creates an instance over Span(int), which stores its argument: refused where the expansion
// runs it, as a compile diagnostic at the expansion.
object Macros:
  class Wide extends jlib.Span(3)
  inline def width: Int = ${ widthImpl }
  def widthImpl(using Quotes): Expr[Int] = Expr(new Wide().n)
