// Extension methods defined in a block: found from the block, callable on siblings without a
// receiver, and capturing the locals around them.

object Comp:
  def render(k: Int): String =
    extension (v: Int)
      def scaled: Int = v * k
      def scaledTwice: Int = scaled * 2
    extension (s: String) def shout(n: Int): String = s.toUpperCase + "!" * n
    val xs = List(1, 2, 3).map(_.scaled)
    s"${2.scaled} ${3.scaledTwice} ${"hi".shout(2)} $xs"
@main def main(): Unit =
  println(Comp.render(3))
  val k = 10
  extension (v: Int) def scaled: Int = v * k
  println(2.scaled)
  val f = (x: Int) => x.scaled + 1
  println(f(4))
