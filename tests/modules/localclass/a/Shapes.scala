package lca

// An anonymous class and a named local class of upstream bodies, reached downstream: their
// names are the whole build's, from the origins.
trait Shape:
  def area: Double

object Shapes:
  def square(s: Double): Shape = new Shape:
    def area = s * s
  def describe(n: Int): String =
    class Box(val v: Int):
      def show = "box " + v
    new Box(n).show
