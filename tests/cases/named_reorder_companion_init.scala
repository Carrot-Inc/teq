case class C(x: Int, y: Int)
object C:
  println("init C")

case class D(x: Int, y: Int = 2)
object D:
  println("init D")

case class E(x: Int, y: Int)
object E:
  println("init E")

def mark(s: String): Int =
  println(s)
  0

@main def namedReorderCompanionInit(): Unit =
  val c = C(y = mark("y"), x = mark("x"))
  println(c.x)
  val d = D(x = mark("dx"))
  println(d.y)
  val e = E(mark("ex"), mark("ey"))
  println(e.y)
