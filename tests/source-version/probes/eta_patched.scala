object Counter { var n = 0 }
def side: Int = { Counter.n += 1; 10 * Counter.n }
@main def run(): Unit = {
  val x = 1
  val f = x _
  println(f())
  val g = side _
  println(Counter.n)
  println(g() + g())
  val h = (x + 1) _
  println(h())
  val k: () => Int = x _
  println(k())
}
