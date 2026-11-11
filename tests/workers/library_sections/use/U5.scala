package use

import pairs.Pairs.*

object U5:
  def a: (Int, String) = 5.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 5)
