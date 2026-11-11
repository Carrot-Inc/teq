package use

import marked.Marked

object U3:
  def a: Int = Marked.fresh(3)
  def b(n: Int): Int = Marked.fresh(n) + 3
