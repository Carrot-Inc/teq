class Ops(val s: String):
  def h(a: Int)(b: => Int = { println("hb"); a + 1 }, c: Int = 3): Int = a * 100 + b * 10 + c
extension (s: String)
  def ops: Ops = Ops(s)
  export ops.h
@main def run(): Unit =
  println("x".h(1)(c = 9))
  println("x".h(1)(4, 9))
