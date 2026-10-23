package fp
object S:
  val z0 = List(0)
  val z1 = List(1)
  val z2 = List(2)
  val z3 = List(3)
  val z4 = List(4)
  val z5 = List(5)
  val z6 = List(6)
  def outer(): Int =
    def f(x: Int): Int =
      val x = 3
      if x > 0 then f(x = x - 1) else 0
    f(x = 1)
