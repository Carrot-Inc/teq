def side(msg: String): Int = { println("eval " + msg); msg.length }
class P(val a: Int, val b: Int)
class Q extends P(b = side("bb"), a = side("a"))
class R(x: Int) extends P(b = side("bb") + x, a = side("a"))
@main def run(): Unit =
  val q = Q()
  println(q.a + " " + q.b)
  val r = new P(b = side("bbb"), a = side("aa")) { override def toString = "sum " + (a + b) }
  println(r)
  println(R(10).b)
