package cla

object Dup:
  def v: Int = 1

object One:
  def v: Int = Dup.v
