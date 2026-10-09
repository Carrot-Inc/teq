package lpb

object WithLocalB:
  def make: Int =
    class Loc extends lpa.P
    new Loc().p
