// Two arrays of a class appended and concatenated.
class Box(val v: Int)

@main def run(): Unit =
  val xs = Array(new Box(1))
  val ys = xs :+ new Box(2)
  println((ys ++ xs).length)
