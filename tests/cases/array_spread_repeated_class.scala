// An array of a class spread into a repeated parameter.
class Box(val v: Int)
def cnt(xs: Box*): Int = xs.map(_.v).sum

@main def run(): Unit =
  val arr = Array(new Box(1), new Box(5))
  println(cnt(arr*))
