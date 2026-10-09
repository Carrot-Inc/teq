package use

import lib.*

object Sizes:
  def scaled(x: Int): Int = x * factor
  def pick: Int = if flag then 1 else 2
