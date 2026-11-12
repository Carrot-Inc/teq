// A macro's provider unit runs during the expansion of a later round: dotty's run of
// the provider passes FirstTransform before the run of the suspended consumer calls
// its macro (Driver.scala:53-64), so `Helper.read`'s `x`, a constant val read before
// Helper's initialiser has set it, is its literal there as in the program's run.
import scala.quoted.*
object Helper:
  val before: Int = read
  val x: 42 = 42
  def read: Int = x
object Mac:
  inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(Helper.before)
