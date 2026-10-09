import scala.quoted.*

inline def calc(inline n: Int): Int = ${ calcImpl('n) }

def calcImpl(n: Expr[Int])(using Quotes): Expr[Int] =
  Expr(twice(n.valueOrAbort) + new Box(3).get)
