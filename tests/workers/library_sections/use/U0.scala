package use

import pairs.Pairs.*

object U0:
  def a: (Int, String) = 0.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 0)
