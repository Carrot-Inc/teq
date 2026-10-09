case class ByName(x: => Int)
@main def run(): Unit =
  var n = 0
  val b = ByName({ n += 1; n })
  println(b.x)
  println(b.x)
  println(n)
