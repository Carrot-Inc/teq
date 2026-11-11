case class D(x: Int, y: Int = 0)
object D:
  println("init D")

case class F(x: Int, y: Int)
object F:
  println("init F")

def mark(s: String): Int =
  println(s)
  0

@main def run(): Unit =
  val d = D(y = mark("dy"), x = mark("dx"))
  val f = ({ val z = 0; F }).apply(y = mark("fy"), x = mark("fx"))
  println(d.x + f.x)
