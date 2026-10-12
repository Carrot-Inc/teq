// A macro applying a function value with `Apply`: no method type, refused as dotty refuses it
// (`TypeAssigner.assignType`, `takesNoParamsMsg`); scalac reports the re-typed expansion too (E050).
import scala.quoted.*
object Mac:
  inline def test: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    import quotes.reflect.*
    Apply('{ ((x: Int) => x + 1) }.asTerm, List('{ 2 }.asTerm)).asExprOf[Int]
