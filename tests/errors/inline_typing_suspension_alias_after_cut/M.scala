// `type Bad = Missing` stands after `B.dependent`, where `B` suspends: dotty's traversal never reaches it, and
// its completion owns what it reports, which goes with it.
// expect: Z.scala:2:24: error: type mismatch: found 456, required Boolean
// expect: 1 error found
// absent: Missing
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
