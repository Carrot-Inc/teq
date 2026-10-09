package cells

import scala.quoted.*

object B:
  def m = 2
  final class L:
    val shown = "l" + A.n
  def implB(using Quotes): Expr[String] = Expr(new A.K().shown)

inline def fromB: String = ${ B.implB }
