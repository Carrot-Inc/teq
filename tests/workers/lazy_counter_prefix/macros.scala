import scala.quoted.*

object M:
  def readA(using Quotes): Expr[Int] = Expr(S.a)
  def readB(using Quotes): Expr[Int] = Expr(S.b)

inline def ma: Int = ${ M.readA }
inline def mb: Int = ${ M.readB }
