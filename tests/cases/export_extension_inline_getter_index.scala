class Ops(val s: String):
  inline def p(a: Int, b: Int = 1, inline c: Int = { println("pc"); 2 }): Int = a * 100 + b * 10 + c + s.length * 1000
extension (s: String)
  def ops: Ops = Ops(s)
  export ops.p
@main def run(): Unit =
  println("x".p(1, c = 7))
  println("x".p(1, 2, 3))
