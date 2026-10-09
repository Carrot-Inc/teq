import scala.quoted.*
inline def value: Int = ${ impl }
def impl(using Quotes): Expr[Int] = Expr(Helper.calc)
