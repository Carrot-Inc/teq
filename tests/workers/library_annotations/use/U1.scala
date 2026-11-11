package use

import marked.Marked

object U1:
  def a: Int = Marked.fresh(1)
  def b(n: Int): Int = Marked.fresh(n) + 1
