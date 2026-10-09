package pab

object LocalB:
  def make: paa.Fn =
    class L extends paa.Fn:
      def apply(x: Int): Int = x
    new L
