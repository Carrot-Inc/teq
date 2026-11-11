package custom

import Types.EmptyTuple

object Lib:
  def make: Int *: EmptyTuple = (1, "ok")
  def take(x: Int *: EmptyTuple): String = x.tail.head
