package use

import pairs.Pairs.*

object U6:
  def a: (Int, String) = 6.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 6)
