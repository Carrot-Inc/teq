package mfb

import mfa.F

object B:
  def one: Int = F.twice(3)
  def two: Int = F.twice(4) + F.twice(5)
