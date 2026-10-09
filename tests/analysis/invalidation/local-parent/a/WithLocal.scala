package lpa

object WithLocal:
  def make: Int =
    class Loc extends P
    new Loc().p
