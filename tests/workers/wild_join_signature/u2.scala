import Convs.*
object U2:
  given Ctx = Ctx(true)
  def pad(n: Int): Int = List(1, 2, 3).map(_ + n).foldLeft(0)(_ + _) + List("a", "b").map(_.length).sum
  val p1 = pad(1); val p2 = pad(2); val p3 = pad(3); val p4 = pad(4)
  val y: Int = A.mods
