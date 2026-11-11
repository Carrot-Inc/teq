package use

import pairs.Pairs.*

object U2:
  def a: (Int, String) = 2.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 2)
