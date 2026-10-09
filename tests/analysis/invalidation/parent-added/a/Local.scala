package paa

object Local:
  def make: Fn =
    class L extends Fn:
      def apply(x: Int): Int = x
    new L
