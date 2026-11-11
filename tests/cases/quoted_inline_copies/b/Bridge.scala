package quotedcopies
import scala.quoted.*
object Bridge:
  inline def direct: Int = Macros.nested
  inline def quoted: Int = ${ quotedImpl }
  def quotedImpl(using Quotes): Expr[Int] = '{ Macros.nested + Macros.twice(Macros.number) }
