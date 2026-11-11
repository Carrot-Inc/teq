package use

import marked.Marked

object U6:
  def a: Int = Marked.fresh(6)
  def b(n: Int): Int = Marked.fresh(n) + 6
