package p03
object Fors:
  val xs = List(1, 2)
  val a = for x <- xs yield x + 1
  val b = for x <- xs; y <- xs if x < y yield (x, y)
  val c = for x <- xs; z = x + 1; y <- xs yield x + y + z
  def d(): Unit = for x <- xs do println(x)
  val e = for (u, v) <- List((1, 2)) yield u + v
  val f = for case (w: Int) <- List(1) yield w
