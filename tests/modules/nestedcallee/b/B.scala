package use

object B:
  def run(x: Int): Int = nest.A.Inner.inc(x)
