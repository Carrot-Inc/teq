package use

import marked.Marked

object U4:
  def a: Int = Marked.fresh(4)
  def b(n: Int): Int = Marked.fresh(n) + 4
