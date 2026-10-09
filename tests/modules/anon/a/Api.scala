package ana

trait Counter:
  def next(): Int

object Counters:
  def from(start: Int): Counter =
    var n = start
    new Counter:
      def next(): Int =
        n += 1
        n
  def squares(xs: List[Int]): List[Int] =
    def sq(x: Int) = x * x
    xs.map(sq)
  class Local(val k: Int):
    def plus(f: Int => Int): Int = f(k)
