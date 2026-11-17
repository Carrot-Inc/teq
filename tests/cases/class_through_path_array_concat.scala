// Two arrays of a class named through a path, made by the class's tag, appended and concatenated.
class Outer:
  class Box(val v: Int)

@main def run(): Unit =
  val o = new Outer
  val xs = Array(new o.Box(1))
  val ys = xs :+ new o.Box(2)
  println((ys ++ xs).length)
