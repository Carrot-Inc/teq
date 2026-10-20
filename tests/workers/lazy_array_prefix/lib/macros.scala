import scala.quoted.*

object M:
  def readA(using Quotes): Expr[Int] = Expr(S.a(0))
  def readB(using Quotes): Expr[Int] = Expr(S.b(0))

transparent inline def ma: Int = ${ M.readA }
transparent inline def mb: Int = ${ M.readB }
