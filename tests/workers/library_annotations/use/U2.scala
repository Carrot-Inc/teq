package use

import marked.Marked

object U2:
  def a: Int = Marked.fresh(2)
  def b(n: Int): Int = Marked.fresh(n) + 2
