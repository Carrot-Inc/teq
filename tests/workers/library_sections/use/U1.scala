package use

import pairs.Pairs.*

object U1:
  def a: (Int, String) = 1.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 1)
