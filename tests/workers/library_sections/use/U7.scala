package use

import pairs.Pairs.*

object U7:
  def a: (Int, String) = 7.pair("a")
  def b(n: Int): (String, Int) = "b".pair(n + 7)
