package mtb

import mta.T

object B:
  def picked: Int = T.pick(true)
  def word: String = T.pick(false)
  def first: Runnable = T.runner("first")
  def second: Runnable = T.runner("second")
