package p

import scala.quoted.*

object Squares:
  val memo: scala.collection.mutable.Map[Int, Int] = scala.collection.mutable.Map.empty

  def square(n: Int): Int = memo.getOrElseUpdate(n, n * n)

  def impl(n: Expr[Int])(using Quotes): Expr[Int] = Expr(square(n.valueOrAbort))

inline def sq(inline n: Int): Int = ${ Squares.impl('n) }
