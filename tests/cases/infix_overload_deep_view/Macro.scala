// A macro that counts its expansions, the argument of a deeper selection a view reaches: typed once
// before the views are searched (`viewExists`, Implicits.scala), and once only for the application.
import scala.quoted.*
object M:
  var count: Int = 0
  inline def arg: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    count += 1
    Expr(count)
