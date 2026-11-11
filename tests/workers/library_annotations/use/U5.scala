package use

import marked.Marked

object U5:
  def a: Int = Marked.fresh(5)
  def b(n: Int): Int = Marked.fresh(n) + 5
