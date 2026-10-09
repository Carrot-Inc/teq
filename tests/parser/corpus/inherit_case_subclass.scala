case class P(x: Int)
class Q(x: Int) extends P(x)
@main def run() =
  println(P(1) == new Q(1))
  println(new Q(1) == P(1))
  println(new Q(1) == new Q(1))
  println(new Q(1))
