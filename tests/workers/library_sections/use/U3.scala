package use

import pairs.Pairs.*

object U3:
  def a: (Int, String) = 3.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 3)
