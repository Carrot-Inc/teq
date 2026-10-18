package itb

import ita.*

object Use:
  def pick(x: Int or String): Int = x match
    case Left(i) => i
    case Right(s) => s.length
  def both(x: Int Both String): Int = x.a + x.b.length
