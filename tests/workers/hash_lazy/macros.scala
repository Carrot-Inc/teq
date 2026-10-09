import scala.quoted.*

object M:
  def hash(using Quotes): Expr[Int] = Expr(System.identityHashCode(S))
  def readThenHash(using Quotes): Expr[Int] =
    val v = S.x
    Expr(System.identityHashCode(S) + 0 * v)

inline def h: Int = ${ M.hash }
inline def rh: Int = ${ M.readThenHash }
