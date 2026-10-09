@main def run(): Unit =
  val xs = List(1, 2)
  val n = xs length
  println(n)
  def h(t: (Int, Int)) = t._1 + t._2
  println(h(1, 2))
  val m = Map(1 -> 2)
  println(m + (3, 4))
