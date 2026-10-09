package prb

object LocalB:
  def make: pra.Fn =
    class L extends pra.Fn:
      def apply(x: Int): Int = x
    new L
