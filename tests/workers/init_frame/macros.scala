import scala.quoted.*

object M:
  def readA(using Quotes): Expr[Int] = Expr(S.counters._1())
  def readB(using Quotes): Expr[Int] = Expr(S.counters._2())

inline def ma: Int = ${ M.readA }
inline def mb: Int = ${ M.readB }
