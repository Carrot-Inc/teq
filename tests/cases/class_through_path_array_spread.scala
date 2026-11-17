// An array of a class named through a path, made by the class's tag, spread into a repeated parameter.
class Outer:
  class Box(val v: Int)

@main def run(): Unit =
  val o = new Outer
  def cnt(xs: o.Box*): Int = xs.map(_.v).sum
  val arr = Array(new o.Box(1), new o.Box(5))
  println(cnt(arr*))
