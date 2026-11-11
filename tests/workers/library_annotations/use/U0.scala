package use

import marked.Marked

object U0:
  def a: Int = Marked.fresh(0)
  def b(n: Int): Int = Marked.fresh(n) + 0
