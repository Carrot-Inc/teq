// A map's `filter` and `tail` (`collection.Map`'s, each map class's own) are calls of those
// members alone: a class of the program with a `filter` and a `tail` of its own that nothing
// calls is not kept by them.
class Box(val x: Int):
  def filter(p: Int => Boolean): Box = if p(x) then this else Box(-1)
  def tail: Box = Box(x - 1)
  def get: Int = x

@main def run(): Unit =
  val m: Map[String, Int] = Map("a" -> 1, "b" -> 2)
  println(m.filter(_._2 > 1))
  println(m.tail.size)
  val h = scala.collection.mutable.HashMap("c" -> 3, "d" -> 4)
  println(h.filter(_._2 > 3))
  println(h.tail.size)
  println(Box(3).get)
