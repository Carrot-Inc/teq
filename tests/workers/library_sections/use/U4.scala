package use

import pairs.Pairs.*

object U4:
  def a: (Int, String) = 4.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 4)
