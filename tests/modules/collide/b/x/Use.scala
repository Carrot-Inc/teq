package clb

object B:
  def f: Int =
    class L { def n = 2 }
    new L().n
