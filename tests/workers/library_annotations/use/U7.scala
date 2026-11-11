package use

import marked.Marked

object U7:
  def a: Int = Marked.fresh(7)
  def b(n: Int): Int = Marked.fresh(n) + 7
