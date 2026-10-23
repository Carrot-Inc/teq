package p22
class S(x: Int, val y: Int):
  def this(s: String) = this(s.length, 0)
  def this() =
    this("a")
    println(1)
class G[T](t: T):
  def this(t: T, u: Int) = this(t)
object U:
  val s = new S("q")
  val g = new G[Int](1, 2)
  val g2 = G(1)
