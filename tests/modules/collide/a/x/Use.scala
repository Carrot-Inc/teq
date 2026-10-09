package cla

object A:
  def f: Int =
    class L { def n = 1 }
    new L().n
